package core

import (
	"context"
	"os"
	"path/filepath"

	"komorebi-server/src/db"
	"komorebi-server/src/downloaders"
	"komorebi-server/src/workers"
)

func Defer() {
	workers.CloseScheduler()

	ctx := context.Background()
	jobs := downloaders.GetActiveJobs()
	if len(jobs) > 0 {
		db.UpdateDownloadJobs(ctx, jobs...)
	}

	downloaders.TorrentClient().Close()

	// Remove temp dir
	tempDir := filepath.Join(os.TempDir(), "KOMOREBI")
	os.RemoveAll(tempDir)
}
