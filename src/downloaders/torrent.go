package downloaders

import (
	"context"
	"errors"
	"os"
	"sync"
	"time"
	"uuid"

	"komorebi-server/src/db"

	"komorebi-server/src/models"

	"komorebi-server/configs"
	"komorebi-server/src/workers"

	"github.com/cenkalti/backoff/v7"
	"github.com/cenkalti/rain/v2/torrent"
	"github.com/go-co-op/gocron/v2"
	zlog "github.com/rs/zerolog/log"
	"github.com/samber/lo"
)

type torrentDownloader struct {
	ActiveItems map[uuid.UUID]*torrent.Torrent
	ActiveJobs  map[uuid.UUID]*models.DownloadJob
	muItem      sync.RWMutex
	muJob       sync.RWMutex
}

var TorrentClient = sync.OnceValue(func() (cl *torrent.Session) {
	config := torrent.DefaultConfig

	config.DataDir = configs.GetConfig().Env.VaultLoc
	config.CustomLogHandler = customTorrentLogger{}

	s, err := torrent.NewSession(config)
	if err != nil {
		zlog.Err(err).Msg("Failed to initialize torrent session")
		return nil
	}
	return s
})

func (d *torrentDownloader) createUpdater() workers.JobUpdater {
	return func(id uuid.UUID, updateFn func(*models.DownloadJob)) {
		d.muJob.Lock()
		job, ok := d.ActiveJobs[id]
		if ok {
			updateFn(job)
			if job.Status == models.DownloadStatusCompleted {
				delete(d.ActiveJobs, id)
			}
		}
		d.muJob.Unlock()

		if ok && job.Status == models.DownloadStatusCompleted {
			d.muItem.Lock()
			delete(d.ActiveItems, id)
			d.muItem.Unlock()
		}
	}
}

func (d *torrentDownloader) Submit(ctx context.Context, job *models.DownloadJob) (string, error) {
	log := zlog.With().Type("downloader", d).
		Str("job url", lo.Substring(job.Url, 0, 25)+"...").
		Str("job loc", job.Location).Logger()

	t, err := TorrentClient().AddURI(job.Url, &torrent.AddTorrentOptions{
		ID:                job.Id.String(),
		StopAfterDownload: true,
	})
	if err != nil {
		log.Err(err).Msg("Failed to add torrent")
	} else {
		log.Info().Str("torrent id", t.ID()).Msg("Added torrent")
	}

	job.EngineId = t.ID()

	d.muItem.Lock()
	d.ActiveItems[job.Id] = t
	d.muItem.Unlock()

	d.muJob.Lock()
	d.ActiveJobs[job.Id] = job
	d.muJob.Unlock()

	db.UpdateDownloadJobs(ctx, *job)

	t.Start()

	workers.AddJobWithId(gocron.DurationJob(time.Second*2),
		gocron.NewTask(workers.TrackTorrentDownload, ctx, job.Id, t, d.createUpdater()), job.Id)

	return job.Id.String(), nil
}

func (d *torrentDownloader) Pause(ctx context.Context, job *models.DownloadJob) error {
	log := zlog.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	t := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if t == nil {
		msg := "no active download found"
		log.Error().Msg(msg)
		return errors.New(msg)
	}

	t.Stop()

	// Stop the worker
	err := workers.RemoveJob(job.Id)
	if err != nil {
		log.Err(err).Msg("failed to remove job")
	}

	log.Debug().Msg("Download Paused")

	d.muJob.Lock()
	activeJob := d.ActiveJobs[job.Id]
	d.muJob.Unlock()
	if activeJob != nil {
		activeJob.Status = models.DownloadStatusPaused
		db.UpdateDownloadJobs(ctx, *activeJob)
	}
	log.Debug().Msg("Download Paused")
	return nil
}

func (d *torrentDownloader) Resume(ctx context.Context, job *models.DownloadJob) error {
	logger := zlog.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	t := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if t == nil {
		msg := "not an active job"
		err := errors.New(msg)
		logger.Err(err).Msg(msg)
		return err
	}

	t.Start()

	workers.AddJobWithId(gocron.DurationJob(time.Second*2),
		gocron.NewTask(workers.TrackTorrentDownload, ctx, job.Id, t, d.createUpdater()), job.Id)

	d.muJob.RLock()
	j := d.ActiveJobs[job.Id]
	d.muJob.RUnlock()
	if j != nil {
		j.Status = models.DownloadStatusDownloading
		db.UpdateDownloadJobs(ctx, *j)
	}

	logger.Debug().Msg("Resumed download")
	return nil
}

func (d *torrentDownloader) Delete(ctx context.Context, job *models.DownloadJob) error {
	log := zlog.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	t := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if t == nil {
		msg := "no active download found"
		log.Error().Msg(msg)
		return errors.New(msg)
	}

	t.Stop()

	err := workers.RemoveJob(job.Id)
	if err != nil {
		log.Err(err).Msg("failed to remove job")
	}

	log.Debug().Msg("Download Cancelled")

	d.muItem.Lock()
	delete(d.ActiveItems, job.Id)
	d.muItem.Unlock()

	d.muJob.RLock()
	j := d.ActiveJobs[job.Id]
	d.muJob.RUnlock()
	if j != nil {
		j.Status = models.DownloadStatusDeleted
		db.UpdateDownloadJobs(ctx, *j)
	}

	d.muJob.Lock()
	delete(d.ActiveJobs, job.Id)
	d.muJob.Unlock()

	loc := job.Location
	TorrentClient().RemoveTorrent(t.ID(), true)

	// Retrying in case os takes time to release lock (e.g. Windows Defender)
	go func(loc string) {
		_, err := backoff.Retry(ctx, func() (any, error) {
			return nil, os.RemoveAll(loc)
		}, backoff.WithMaxTries(5))
		if err != nil {
			log.Debug().Err(err).Str("loc", loc).Msg("failed to delete files after retries")
		}
	}(loc)

	log.Debug().Msg("Removed torrent")
	return nil
}

func (d *torrentDownloader) Status(_ context.Context) ([]models.DownloadJob, error) {
	d.muJob.RLock()
	defer d.muJob.RUnlock()

	jobs := make([]models.DownloadJob, 0, len(d.ActiveJobs))
	for _, v := range d.ActiveJobs {
		jobs = append(jobs, *v)
	}
	return jobs, nil
}

func (d *torrentDownloader) RestoreJob(ctx context.Context, job *models.DownloadJob) {
	// rain re-loads torrents by their ID from its own bolt store on session start.
	t := TorrentClient().GetTorrent(job.EngineId) // look up by ID
	if t == nil {
		zlog.Warn().Any("id", job.Id).Msg("torrent not found in rain session — marking error")

		_, err := d.Submit(ctx, job)
		if err != nil {
			job.Status = models.DownloadStatusError
		}
		db.UpdateDownloadJobs(ctx, *job)
		return
	}

	d.muItem.Lock()
	d.ActiveItems[job.Id] = t
	d.muItem.Unlock()

	d.muJob.Lock()
	d.ActiveJobs[job.Id] = job
	d.muJob.Unlock()

	if job.Status == models.DownloadStatusPaused || job.Status == models.DownloadStatusError {
		return // don't auto-resume
	}

	t.Start()

	workers.AddJobWithId(gocron.DurationJob(time.Second*2),
		gocron.NewTask(workers.TrackTorrentDownload, ctx, job.Id, t, d.createUpdater()), job.Id)
}

func (d *torrentDownloader) CleanupOrphans() {
	d.muJob.RLock()
	activeIds := make(map[string]bool)
	for _, job := range d.ActiveJobs {
		activeIds[job.EngineId] = true
	}
	d.muJob.RUnlock()

	for _, t := range TorrentClient().ListTorrents() {
		if !activeIds[t.ID()] {
			zlog.Info().Str("id", t.ID()).Msg("Removing orphaned torrent from rain session")
			TorrentClient().RemoveTorrent(t.ID(), true)
		}
	}
}
