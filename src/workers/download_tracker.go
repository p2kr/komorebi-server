package workers

import (
	"context"
	"errors"
	"time"
	"uuid"

	"komorebi-server/src/db"

	"komorebi-server/src/models"

	"github.com/cavaliergopher/grab/v3"
	"github.com/cenkalti/rain/v2/torrent"
	"github.com/go-co-op/gocron/v2"
	zlog "github.com/rs/zerolog/log"
)

// JobUpdater is a callback provided by the downloader to safely update a job's state.
// In the future, this can be replaced by a database repository interface (e.g., db.SaveJob).
type JobUpdater func(id uuid.UUID, updateFn func(*models.DownloadJob))

func TrackDirectDownload(ctx context.Context, id uuid.UUID, resp *grab.Response, updater JobUpdater) {
	var isComplete bool
	var err error
	var j models.DownloadJob

	updater(id, func(job *models.DownloadJob) {
		job.Status = models.DownloadStatusDownloading
		job.DownloadSpeed = int64(resp.BytesPerSecond())
		job.DownloadedSize = resp.BytesComplete()

		eta := resp.ETA()
		if !eta.IsZero() {
			job.EtaSec = int64(time.Until(eta).Seconds())
		}
		job.Progress = resp.Progress() * 100

		if resp.IsComplete() {
			isComplete = true
			err = resp.Err()

			if errors.Is(err, context.Canceled) {
				job.Status = models.DownloadStatusPaused
			} else if err != nil {
				job.Status = models.DownloadStatusError
			} else {
				job.Status = models.DownloadStatusCompleted
				job.Progress = 100
			}

			job.DownloadSpeed = 0
			job.EtaSec = 0

			j = *job
		}
	})

	if isComplete {
		zlog.Err(err).Any("id", id).Msg("job completed")
		db.UpdateDownloadJobs(ctx, j)
		RemoveJob(id)

		// Start Postprocess
		AddJob(gocron.OneTimeJob(
			gocron.OneTimeJobStartImmediately(),
		), gocron.NewTask(PostDownload, j))
	}
}

func TrackTorrentDownload(ctx context.Context, id uuid.UUID, t *torrent.Torrent, updater JobUpdater) {
	log := zlog.With().Any("id", id).Logger()
	isComplete := false

	select {
	case <-t.NotifyComplete():
		isComplete = true
	default:
	}

	var err error
	var j models.DownloadJob

	updater(id, func(job *models.DownloadJob) {
		job.Status = models.DownloadStatusDownloading
		stats := t.Stats()

		if stats.Bytes.Total == 0 {
			job.Status = models.DownloadStatusQueued
			job.DownloadSpeed = 0
			job.EtaSec = -1
			return
		}

		job.DownloadSpeed = int64(stats.Speed.Download)
		job.DownloadedSize = stats.Bytes.Completed

		job.TotalSize = stats.Bytes.Total

		if job.TotalSize > 0 {
			job.Progress = float64(job.DownloadedSize) / float64(job.TotalSize) * 100.0
		}

		if stats.ETA != nil {
			job.EtaSec = int64(stats.ETA.Seconds())
		} else {
			job.EtaSec = -1
		}

		err = stats.Error
		if err != nil {
			job.Status = models.DownloadStatusError
		}

		if isComplete {
			job.Status = models.DownloadStatusCompleted
			job.Progress = 100
			job.DownloadSpeed = 0
			job.EtaSec = 0
		}
		j = *job
	})

	if isComplete || err != nil {
		log.Err(err).Msg("job completed")
		db.UpdateDownloadJobs(ctx, j)
		RemoveJob(id)

		// Start Postprocess
		AddJob(gocron.OneTimeJob(
			gocron.OneTimeJobStartImmediately(),
		), gocron.NewTask(PostDownload, j))
	}
}
