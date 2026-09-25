package downloaders

import (
	"cmp"
	"context"
	"errors"
	"net/url"
	"slices"
	"uuid"

	"komorebi-server/src/workers"

	"komorebi-server/src/db"

	"komorebi-server/src/models"

	"github.com/cavaliergopher/grab/v3"
	"github.com/cenkalti/rain/v2/torrent"
	"github.com/go-co-op/gocron/v2"
	zlog "github.com/rs/zerolog/log"
	"github.com/samber/lo"
	"gorm.io/gorm"
)

// Downloader defines the contract for any download client (HTTP, Torrent, Aria2, etc.)
type Downloader interface {
	// Submit Add submits a new URL (Magnet, HTTP, etc.) to the downloader.
	// It returns a unique ID for the download job.
	Submit(ctx context.Context, job *models.DownloadJob) (string, error)

	// Pause Action methods acting on a specific download job by its ID.
	Pause(ctx context.Context, job *models.DownloadJob) error
	Resume(ctx context.Context, job *models.DownloadJob) error
	Delete(ctx context.Context, job *models.DownloadJob) error

	// Status returns all active and completed downloads known to the client.
	Status(ctx context.Context) ([]models.DownloadJob, error)

	// RestoreJob restores a download job from the database
	RestoreJob(ctx context.Context, job *models.DownloadJob)

	// CleanupOrphans cleans up independent states like orphaned torrents in the client
	CleanupOrphans()
}

var (
	direct = &directDownloader{
		ActiveItems: make(map[uuid.UUID]*grab.Response),
		ActiveJobs:  make(map[uuid.UUID]*models.DownloadJob),
	}
	torr = &torrentDownloader{
		ActiveItems: make(map[uuid.UUID]*torrent.Torrent),
		ActiveJobs:  make(map[uuid.UUID]*models.DownloadJob),
	}
)

func GetDownloader(u string) (Downloader, error) {
	log := zlog.With().Str("url", lo.Substring(u, 0, 25)).Logger()
	ur, err := url.Parse(u)
	if err != nil {
		log.Err(err).Msg("Unparsable URL")
		return nil, err
	}

	switch ur.Scheme {
	case "http", "https":
		return direct, nil
	case "magnet":
		return torr, nil
	default:
		return nil, errors.New("unsupported URL")
	}
}

func GetJobById(id uuid.UUID) (*models.DownloadJob, Downloader) {
	job := direct.ActiveJobs[id]
	if job != nil {
		return job, direct
	}
	job = torr.ActiveJobs[id]
	if job != nil {
		return job, torr
	}

	return nil, nil
}

func GetActiveJobs() []models.DownloadJob {
	var jobs []models.DownloadJob
	ctx := context.Background()

	d, _ := direct.Status(ctx)
	jobs = append(jobs, d...)

	t, _ := torr.Status(ctx)
	jobs = append(jobs, t...)

	// Sort jobs according to eta
	slices.SortFunc(jobs, func(left, right models.DownloadJob) int {
		return cmp.Compare(left.EtaSec, right.EtaSec)
	})

	return jobs
}

func RestoreJobs(ctx context.Context) {
	jobs, err := gorm.G[models.DownloadJob](db.GetDb()).Where("status not in ?",
		[]models.DownloadStatus{
			models.DownloadStatusDeleted,
			models.DownloadStatusReady, models.DownloadStatusPartial, models.DownloadStatusProcessed,
		}).Find(ctx)
	if err != nil {
		zlog.Error().Err(err).Msg("Failed to restore jobs")
	}
	restored := 0

	completedJobs, incompleteJobs := lo.FilterReject(jobs, func(j models.DownloadJob, i int) bool {
		return j.Status == models.DownloadStatusCompleted || j.Status == models.DownloadStatusProcessing
	})

	for _, job := range incompleteJobs {
		downloader, err := GetDownloader(job.Url)
		if err != nil {
			zlog.Error().Any("id", job.Id).Err(err).Msg("Failed to restore job")
			continue
		}
		restored += 1
		downloader.RestoreJob(ctx, &job)
	}

	// Restore processing jobs
	workers.AddJob(gocron.OneTimeJob(gocron.OneTimeJobStartImmediately()), gocron.NewTask(
		func() {
			workers.PostDownload(completedJobs...)
		},
	))

	// Clean up any independent states (e.g., deleted torrents in rain session)
	direct.CleanupOrphans()
	torr.CleanupOrphans()

	zlog.Info().Int("total", len(jobs)).Int("restored", restored).Msg("Restored jobs")
}
