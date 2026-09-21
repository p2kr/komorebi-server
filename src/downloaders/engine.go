package downloaders

import (
	"context"
	"errors"
	"net/url"
	"uuid"

	"komorebi-server/src/models"

	"github.com/cavaliergopher/grab/v3"
	"github.com/cenkalti/rain/v2/torrent"
	"github.com/rs/zerolog/log"
	"github.com/samber/lo"
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
	log := log.With().Str("url", lo.Substring(u, 0, 25)).Logger()
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

	return jobs
}
