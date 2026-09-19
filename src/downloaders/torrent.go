package downloaders

import (
	"sync"
	"uuid"

	"komorebi-server/src/dto"
)

type torrentDownloader struct {
	ActiveJobs map[uuid.UUID]*dto.DownloadJob
	muItem     sync.RWMutex
	muJob      sync.RWMutex
}
