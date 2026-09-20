package downloaders

import (
	"context"
	"errors"
	"log/slog"
	"os"
	"sync"
	"time"
	"uuid"

	"komorebi-server/configs"
	"komorebi-server/src/dto"
	"komorebi-server/src/workers"

	"github.com/anacrolix/torrent"
	"github.com/anacrolix/torrent/storage"
	"github.com/cenkalti/backoff/v7"
	"github.com/go-co-op/gocron/v2"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
	"github.com/samber/lo"
)

type torrentDownloader struct {
	ActiveItems   map[uuid.UUID]*torrent.Torrent
	ActiveStorage map[uuid.UUID]storage.ClientImplCloser
	ActiveJobs    map[uuid.UUID]*dto.DownloadJob
	muItem        sync.RWMutex
	muJob         sync.RWMutex
}

var torrentClient = sync.OnceValue(func() (cl *torrent.Client) {
	config := torrent.NewDefaultClientConfig()
	torrentLogger := log.Logger.Level(zerolog.WarnLevel)
	if configs.GetConfig().TorrentClient.Debug {
		config.Debug = true
		torrentLogger = log.Logger.Level(zerolog.DebugLevel)
	}
	config.Slogger = slog.New(zerolog.NewSlogHandler(torrentLogger))

	dir, err := os.MkdirTemp("", "komorebi-torrent-*")
	if err == nil {
		config.DataDir = dir
	}

	cl, err = torrent.NewClient(config)
	if err != nil {
		log.Err(err).Msg("Failed to initialize torrent client")
		return nil
	}
	return cl
})

func (d *torrentDownloader) createUpdater() workers.JobUpdater {
	return func(id uuid.UUID, updateFn func(*dto.DownloadJob)) {
		d.muJob.Lock()
		defer d.muJob.Unlock()
		if job, ok := d.ActiveJobs[id]; ok {
			updateFn(job)
		}
	}
}

func (d *torrentDownloader) Submit(ctx context.Context, job *dto.DownloadJob) (string, error) {
	log := log.With().Type("downloader", d).
		Str("job url", lo.Substring(job.Url, 0, 25)+"...").Str("job loc", job.Location).Logger()

	spec, _ := torrent.TorrentSpecFromMagnetUri(job.Url)
	s := storage.NewFile(job.Location)
	spec.Storage = s
	t, _, err := torrentClient().AddTorrentSpec(spec)
	if err != nil {
		log.Err(err).Msg("Failed to add torrent")
		return "", err
	}
	job.EngineId = t.InfoHash().String()

	d.muItem.Lock()
	d.ActiveItems[job.Id] = t
	d.ActiveStorage[job.Id] = s
	d.muItem.Unlock()

	d.muJob.Lock()
	d.ActiveJobs[job.Id] = job
	d.muJob.Unlock()

	go func() {
		<-t.GotInfo()
		t.DownloadAll()
	}()

	workers.AddJobWithId(gocron.DurationJob(time.Second*2),
		gocron.NewTask(workers.TrackTorrentDownload, ctx, job.Id, t, d.createUpdater()), job.Id)

	return job.Id.String(), nil
}

func (d *torrentDownloader) Pause(ctx context.Context, job *dto.DownloadJob) error {
	log := log.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	t := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if t == nil {
		msg := "No active download found"
		log.Error().Msg(msg)
		return errors.New(msg)
	}

	t.DisallowDataDownload()
	t.DisallowDataUpload()

	// Stop the worker
	err := workers.RemoveJob(job.Id)
	if err != nil {
		log.Err(err).Msg("failed to remove job")
	}

	log.Debug().Msg("Download Paused")

	d.muJob.Lock()
	if activeJob, ok := d.ActiveJobs[job.Id]; ok {
		activeJob.Status = dto.StatusPaused
	}
	d.muJob.Unlock()

	return nil
}

func (d *torrentDownloader) Resume(ctx context.Context, job *dto.DownloadJob) error {
	log := log.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	t := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if t == nil {
		msg := "not an active job"
		err := errors.New(msg)
		log.Err(err).Msg(msg)
		return err
	}

	t.AllowDataDownload()
	t.AllowDataUpload()

	workers.AddJobWithId(gocron.DurationJob(time.Second*2),
		gocron.NewTask(workers.TrackTorrentDownload, ctx, job.Id, t, d.createUpdater()), job.Id)

	return nil
}

func (d *torrentDownloader) Delete(ctx context.Context, job *dto.DownloadJob) error {
	log := log.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	t := d.ActiveItems[job.Id]
	s := d.ActiveStorage[job.Id]
	d.muItem.RUnlock()

	if t == nil {
		msg := "No active download found"
		log.Error().Msg(msg)
		return errors.New(msg)
	}

	t.Drop()

	err := workers.RemoveJob(job.Id)
	if err != nil {
		log.Err(err).Msg("failed to remove job")
	}

	log.Debug().Msg("Download Cancelled")

	d.muItem.Lock()
	delete(d.ActiveItems, job.Id)
	delete(d.ActiveStorage, job.Id)
	d.muItem.Unlock()

	d.muJob.Lock()
	delete(d.ActiveJobs, job.Id)
	d.muJob.Unlock()

	// Remove files asynchronously: TODO: Not working rn
	loc := job.Location
	go func(loc string, t *torrent.Torrent, s storage.ClientImplCloser) {
		<-t.Closed()
		if s != nil {
			s.Close()
		}

		_, err := backoff.Retry(ctx, func() (any, error) {
			return nil, os.RemoveAll(loc)
		}, backoff.WithMaxTries(5))
		if err != nil {
			log.Debug().Err(err).Str("loc", loc).Msg("failed to delete files after retries")
		}
	}(loc, t, s)

	return nil
}

func (d *torrentDownloader) Status(ctx context.Context) ([]dto.DownloadJob, error) {
	d.muJob.RLock()
	defer d.muJob.RUnlock()

	jobs := make([]dto.DownloadJob, 0, len(d.ActiveJobs))
	for _, v := range d.ActiveJobs {
		jobs = append(jobs, *v)
	}
	return jobs, nil
}
