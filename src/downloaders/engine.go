package downloaders

import (
	"context"
	"errors"
	"net/url"
	"uuid"

	"komorebi-server/src/dto"

	"github.com/cavaliergopher/grab/v3"
	"github.com/rs/zerolog/log"
	"github.com/samber/lo"
)

// Downloader defines the contract for any download client (HTTP, Torrent, Aria2, etc.)
type Downloader interface {
	// Add submits a new URL (Magnet, HTTP, etc.) to the downloader.
	// It returns a unique ID for the download job.
	Submit(ctx context.Context, job *dto.DownloadJob) (string, error)

	// Action methods acting on a specific download job by its ID.
	Pause(ctx context.Context, job *dto.DownloadJob) error
	Resume(ctx context.Context, job *dto.DownloadJob) error
	Delete(ctx context.Context, job *dto.DownloadJob) error

	// Status returns all active and completed downloads known to the client.
	Status(ctx context.Context) ([]dto.DownloadJob, error)
}

var (
	direct = &directDownloader{
		ActiveItems: make(map[uuid.UUID]*grab.Response),
		ActiveJobs:  make(map[uuid.UUID]*dto.DownloadJob),
	}
	torrent = &torrentDownloader{
		ActiveJobs: make(map[uuid.UUID]*dto.DownloadJob),
	}
)

func GetDownloader(u string) (Downloader, error) {
	log := log.With().Str("url", lo.Substring(u, 0, 15)).Logger()
	ur, err := url.Parse(u)
	if err != nil {
		log.Err(err).Msg("Unparsable URL")
		return nil, err
	}

	switch ur.Scheme {
	case "http", "https":
		return direct, nil
	case "magnet":
		// return &torrent, nil
		return nil, nil
	default:
		return nil, errors.New("Unsupported URL")
	}
}

func GetJobById(id uuid.UUID) (*dto.DownloadJob, Downloader) {
	job := direct.ActiveJobs[id]
	if job != nil {
		return job, direct
	}
	job = torrent.ActiveJobs[id]
	if job != nil {
		// return job, torrent
		return job, nil
	}

	return nil, nil
}

func GetActiveJobs() []dto.DownloadJob {
	var jobs []dto.DownloadJob
	ctx := context.Background()

	d, _ := direct.Status(ctx)
	jobs = append(jobs, d...)

	// t, _ := torrent.Status(ctx)
	// jobs = append(jobs, t)

	return jobs
}
