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

func SaveVaultItems(ctx context.Context, items ...models.VaultItem) {
	if len(items) == 0 {
		return
	}

	downloadJobIds := lo.Map(items, func(item models.VaultItem, _ int) uuid.UUID {
		return item.DownloadJobId
	})

	err := appDb.WithContext(ctx).Where("download_job_id in ?", downloadJobIds).Delete(&models.VaultItem{}).Error
	if err != nil {
		zlog.Err(err).Int("count", len(downloadJobIds)).Msg("Failed to delete vault items")
	}

	err = appDb.WithContext(ctx).CreateInBatches(&items, len(items)).Error
	if err != nil {
		zlog.Err(err).Int("count", len(items)).Msg("Failed to save vault items")
	}

	// Update job status to processed status
	err = appDb.WithContext(ctx).Table("download_jobs").
		Where("id in ?", downloadJobIds).Update("status", models.DownloadStatusProcessed).Error

	if err != nil {
		zlog.Err(err).Int("count", len(items)).Msg("Failed to update vault items in db")
	} else {
		zlog.Info().Int("count", len(items)).Msg("Saving vault items in db")
	}
}
