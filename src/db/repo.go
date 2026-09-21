package db

import (
	"context"
	"uuid"

	"komorebi-server/src/models"

	zlog "github.com/rs/zerolog/log"
	"github.com/samber/lo"
	"gorm.io/gorm/clause"
)

func UpdateDownloadJobs(ctx context.Context, jobs ...models.DownloadJob) {
	if len(jobs) == 0 {
		return
	}

	jobs = lo.Filter(jobs, func(job models.DownloadJob, _ int) bool {
		return job.Id != uuid.Nil()
	})

	err := appDb.Clauses(
		clause.OnConflict{Columns: []clause.Column{{Name: "id"}}, UpdateAll: true},
	).WithContext(ctx).CreateInBatches(jobs, len(jobs)).Error
	if err != nil {
		zlog.Err(err).Msg("Failed to update download job")
	}
}
