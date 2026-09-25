package models

import (
	"os"
	"path/filepath"
	"uuid"
)

type DownloadStatus string

const (
	DownloadStatusQueued      DownloadStatus = "QUEUED"
	DownloadStatusDownloading DownloadStatus = "DOWNLOADING"
	DownloadStatusPaused      DownloadStatus = "PAUSED"
	DownloadStatusCompleted   DownloadStatus = "COMPLETED"
	DownloadStatusProcessing  DownloadStatus = "PROCESSING"
	DownloadStatusProcessed   DownloadStatus = "PROCESSED"
	DownloadStatusReady       DownloadStatus = "READY"
	DownloadStatusPartial     DownloadStatus = "PARTIAL"
	DownloadStatusError       DownloadStatus = "ERROR"
	DownloadStatusDeleted     DownloadStatus = "DELETED"
)

// DownloadJob represents a snapshot of a download's current progress
type DownloadJob struct {
	Model `tstype:",extends"`

	Name string `json:"name"`
	// specific id of each engine
	EngineId string `json:"engine_id"`
	// Download location
	Location string `json:"location"`
	// Url. Also serves as an identifier for the download engine
	Url string `json:"url"`
	// Current state of the download
	Status DownloadStatus `json:"status"`
	// Percentage completed (0.0 to 100.0)
	Progress float64 `json:"progress"`
	// Bytes downloaded so far
	DownloadedSize int64 `json:"downloaded_size"`
	// Total size in bytes
	TotalSize int64 `json:"total_size"`
	// Current speed in bytes per second
	DownloadSpeed int64 `json:"download_speed" gorm:"-"`
	// Eta in sec. Negative mean unknown
	EtaSec int64 `json:"eta_sec" gorm:"-"`

	VaultItems []VaultItem `json:"vault_items,omitempty" gorm:"constraint:OnDelete:CASCADE;"`
}

func NewDownloadJob(engineId, location, url, name string) DownloadJob {
	id := uuid.NewV7()

	loc := filepath.Join(location, id.String())
	loc, err := filepath.Abs(loc)
	if err != nil {
		loc = filepath.Join(location, id.String())
		loc = filepath.Clean(loc)
	}
	loc += string(filepath.Separator)

	os.MkdirAll(loc, os.ModePerm)

	return DownloadJob{
		Id:       id,
		Name:     name,
		EngineId: engineId,
		Location: loc,
		Url:      url,
		Status:   DownloadStatusQueued,
		EtaSec:   -1,
	}
}
