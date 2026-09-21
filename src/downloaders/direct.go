package downloaders

import (
	"context"
	"errors"
	"os"
	"sync"
	"time"
	"uuid"

	"komorebi-server/src/models"

	"komorebi-server/src/workers"

	"github.com/cavaliergopher/grab/v3"
	"github.com/go-co-op/gocron/v2"
	"github.com/rs/zerolog/log"
	"github.com/samber/lo"
)

type directDownloader struct {
	ActiveItems map[uuid.UUID]*grab.Response
	ActiveJobs  map[uuid.UUID]*models.DownloadJob
	muItem      sync.RWMutex
	muJob       sync.RWMutex
}

var DirectClient = sync.OnceValue(func() *grab.Client {
	c := grab.NewClient()
	return c
})

func (d *directDownloader) createUpdater() workers.JobUpdater {
	return func(id uuid.UUID, updateFn func(*models.DownloadJob)) {
		d.muJob.Lock()
		defer d.muJob.Unlock()
		if job, ok := d.ActiveJobs[id]; ok {
			updateFn(job)
		}
	}
}

func (d *directDownloader) Submit(ctx context.Context, job *models.DownloadJob) (string, error) {
	log := log.With().Type("downloader", d).
		Str("job url", lo.Substring(job.Url, 0, 25)+"...").Str("job loc", job.Location).Logger()
	req, err := grab.NewRequest(job.Location, job.Url)
	if err != nil {
		log.Err(err).Msg("Failed to submit direct download")
		return "", err
	}

	if ctx != nil {
		req.WithContext(ctx)
	}

	resp := DirectClient().Do(req)

	d.muItem.Lock()
	d.ActiveItems[job.Id] = resp
	d.muItem.Unlock()

	d.muJob.Lock()
	d.ActiveJobs[job.Id] = job
	d.muJob.Unlock()

	log.Debug().Any("response", resp).Msg("Started download")

	workers.AddJobWithId(gocron.DurationJob(time.Second*2),
		gocron.NewTask(workers.TrackDirectDownload, ctx, job.Id, resp, d.createUpdater()), job.Id)

	return job.Id.String(), nil
}

func (d *directDownloader) Pause(ctx context.Context, job *models.DownloadJob) error {
	log := log.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	resp := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if resp == nil {
		msg := "No active download found"
		log.Error().Msg(msg)
		return errors.New(msg)
	}

	err := resp.Cancel()

	// Stop the worker
	err2 := workers.RemoveJob(job.Id)
	if err2 != nil {
		log.Err(err2).Msg("failed to remove job")
	}

	if err == nil || errors.Is(err, context.Canceled) {
		log.Debug().Msg("Download Paused")

		d.muJob.Lock()
		if activeJob, ok := d.ActiveJobs[job.Id]; ok {
			activeJob.Status = models.StatusPaused
		}
		d.muJob.Unlock()

		return nil
	} else {
		log.Err(err).Msg("Error cancelling download")
		return err
	}
}

func (d *directDownloader) Resume(ctx context.Context, job *models.DownloadJob) error {
	log := log.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	resp := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if resp == nil {
		// Prevent accidental redownload
		msg := "not an active job"
		err := errors.New(msg)
		log.Err(err).Msg(msg)
		return err
	}

	resp = DirectClient().Do(resp.Request.WithContext(ctx))

	d.muItem.Lock()
	d.ActiveItems[job.Id] = resp
	d.muItem.Unlock()

	workers.AddJobWithId(gocron.DurationJob(time.Second*2),
		gocron.NewTask(workers.TrackDirectDownload, ctx, job.Id, resp, d.createUpdater()), job.Id)

	return nil
}

func (d *directDownloader) Delete(ctx context.Context, job *models.DownloadJob) error {
	log := log.With().Any("id", job.Id).Type("downloader", d).Logger()

	d.muItem.RLock()
	resp := d.ActiveItems[job.Id]
	d.muItem.RUnlock()

	if resp == nil {
		msg := "No active download found"
		log.Error().Msg(msg)
		return errors.New(msg)
	}

	err := resp.Cancel()
	if err == nil || errors.Is(err, context.Canceled) {
		log.Debug().Msg("Download Cancelled")

		d.muItem.Lock()
		delete(d.ActiveItems, job.Id)
		d.muItem.Unlock()

		d.muJob.Lock()
		delete(d.ActiveJobs, job.Id)
		d.muJob.Unlock()

		// Remove files
		loc := job.Location
		if err := os.RemoveAll(loc); err != nil {
			log.Debug().Err(err).Str("loc", loc).Msg("failed to delete files")
		}

		return nil
	} else {
		log.Err(err).Msg("Error cancelling download")
		return err
	}
}

func (d *directDownloader) Status(ctx context.Context) ([]models.DownloadJob, error) {
	d.muJob.RLock()
	defer d.muJob.RUnlock()

	jobs := make([]models.DownloadJob, 0, len(d.ActiveJobs))
	for _, v := range d.ActiveJobs {
		jobs = append(jobs, *v)
	}
	return jobs, nil
}
