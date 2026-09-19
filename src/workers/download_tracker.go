package workers

import (
	"context"
	"errors"
	"time"
	"uuid"

	"komorebi-server/src/dto"

	"github.com/cavaliergopher/grab/v3"
	"github.com/rs/zerolog/log"
)

// JobUpdater is a callback provided by the downloader to safely update a job's state.
// In the future, this can be replaced by a database repository interface (e.g. db.SaveJob).
type JobUpdater func(id uuid.UUID, updateFn func(*dto.DownloadJob))

func TrackDirectDownload(ctx context.Context, id uuid.UUID, resp *grab.Response, updater JobUpdater) {
	var isComplete bool
	var err error

	updater(id, func(job *dto.DownloadJob) {
		job.Status = dto.StatusDownloading
		job.DownloadSpeed = int64(resp.BytesPerSecond())
		job.DownloadedSize = resp.BytesComplete()

		eta := resp.ETA()
		if !eta.IsZero() {
			job.EtaSec = int64(eta.Sub(time.Now()).Seconds())
		}
		job.Progress = resp.Progress() * 100

		if resp.IsComplete() {
			isComplete = true
			err = resp.Err()

			if errors.Is(err, context.Canceled) {
				job.Status = dto.StatusPaused
			} else if err != nil {
				job.Status = dto.StatusError
			} else {
				job.Status = dto.StatusCompleted
				job.Progress = 100
			}

			job.DownloadSpeed = 0
			job.EtaSec = 0
		}
	})

	if isComplete {
		log.Err(err).Any("id", id).Msg("job completed")
		RemoveJob(id)
	}
}

func TrackTorrentDownload(ctx context.Context, resp *grab.Response) {
	return
}
