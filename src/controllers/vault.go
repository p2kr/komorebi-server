package controllers

import (
	"context"
	"encoding/json/v2"
	"errors"
	"fmt"
	"net/http"
	"os"
	"time"
	"uuid"

	"komorebi-server/src/db"

	"komorebi-server/configs"

	"komorebi-server/src/downloaders"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/cenkalti/backoff/v7"
	mapset "github.com/deckarep/golang-set/v3"
	"github.com/labstack/echo/v5"
	zlog "github.com/rs/zerolog/log"
	"github.com/samber/lo"
	"gorm.io/gorm"
)

func VaultRoutes(g *echo.Group) {
	r := g.Group("/vault")

	r.POST("/add", Add)
	r.GET("/active", Active)
	r.POST("/pause", Pause)
	r.DELETE("/delete", Delete)
	r.POST("/resume", Resume)
	r.GET("/all", AllVaultItems)
	r.DELETE("/delete_vault_item", DeleteVaultItem)
}

type VaultAddPayload struct {
	CrawlerResult  dto.CrawlerResult `json:"crawler_result"`
	UserId         uuid.UUID         `json:"user_id"`
	ShouldDownload bool              `json:"should_download"`
}

func Add(c *echo.Context) error {
	var params VaultAddPayload
	if err := c.Bind(&params); err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	// ssrf prevention
	err := validateUrl(params.CrawlerResult.Link)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	log := zlog.With().Str("url", params.CrawlerResult.Link).Logger()

	// Validate if already exists
	if !params.ShouldDownload {
		job, err := gorm.G[models.DownloadJob](db.GetDb()).
			Where("url = ? and status not in ?", params.CrawlerResult.Link,
				[]models.DownloadStatus{models.DownloadStatusDeleted}).
			First(c.Request().Context())

		if err == nil && job.Url == params.CrawlerResult.Link {
			return fail(c, http.StatusConflict, errors.New("download job already exists"))
		}

		if err != nil && !errors.Is(err, gorm.ErrRecordNotFound) {
			log.Err(err).Msg("Database query failed")
			return fail(c, http.StatusInternalServerError, errors.New("internal server error"))
		}
	}

	d, err := downloaders.GetDownloader(params.CrawlerResult.Link)
	if err != nil {
		log.Err(err).Msg("Failed to get downloader")
		return fail(c, http.StatusBadRequest, err)
	}

	job := models.NewDownloadJob(uuid.Nil().String(), configs.GetConfig().Env.VaultLoc,
		params.CrawlerResult.Link, params.CrawlerResult.Title)

	_, err = d.Submit(c.Request().Context(), &job)
	if err != nil {
		log.Err(err).Str("location", job.Location).Msg("Failed to submit download job")
		return fail(c, http.StatusInternalServerError, err)
	}
	log.Info().Type("downloader", d).Str("location", job.Location).Msg("successfully added download job")
	return success(c, job, http.StatusAccepted)
}

// Active Gets active download jobs
func Active(c *echo.Context) error {
	resp := c.Response()
	resp.Header().Set("Content-Type", "text/event-stream")
	resp.Header().Set("Cache-Control", "no-cache")
	resp.Header().Set("Connection", "keep-alive")

	resp.WriteHeader(http.StatusOK)

	ct := http.NewResponseController(resp)
	ct.Flush()

	ticker := time.NewTicker(time.Second * 1)
	defer ticker.Stop()

	for {
		select {
		case <-c.Request().Context().Done():
			return nil
		case <-ticker.C:
			jobs := downloaders.GetActiveJobs()

			j, err := json.Marshal(jobs)
			if err != nil {
				return err
			}

			if _, err := fmt.Fprintf(resp, "data: %s\n\n", j); err != nil {
				return err
			}

			ct.Flush()
		}
	}
}

func Delete(c *echo.Context) error {
	// expects job id
	var j models.DownloadJob
	if err := c.Bind(&j); err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	job, d := downloaders.GetJobById(j.Id)
	if job == nil || d == nil {
		return fail(c, http.StatusNotFound, errors.New("unknown job id"))
	}

	ctx := context.Background()
	log := zlog.With().Any("job id", job.Id).
		Str("url", lo.Substring(job.Url, 0, 15)+"...").Type("downloader", d).Logger()
	err := d.Delete(ctx, job)
	if err != nil {
		log.Err(err).Msg("Failed to delete job")
		return fail(c, http.StatusInternalServerError, err)
	}

	log.Info().Msg("Deleted job")

	return success(c, job, http.StatusAccepted)
}

func Pause(c *echo.Context) error {
	// expects job id
	var j models.DownloadJob
	if err := c.Bind(&j); err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	job, d := downloaders.GetJobById(j.Id)
	if job == nil || d == nil {
		return fail(c, http.StatusNotFound, errors.New("Unknown Job Id"))
	}

	ctx := context.Background()
	log := zlog.With().Any("job id", job.Id).Str("url", lo.Substring(job.Url, 0, 15)+"...").Type("downloader", d).Logger()

	err := d.Pause(ctx, job)
	if err != nil {
		log.Err(err).Msg("Failed to pause job")
		return fail(c, http.StatusInternalServerError, err)
	}

	log.Info().Msg("Paused job")

	return success(c, job)
}

func Resume(c *echo.Context) error {
	// expects job id
	var j models.DownloadJob
	if err := c.Bind(&j); err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	job, d := downloaders.GetJobById(j.Id)
	if job == nil || d == nil {
		return fail(c, http.StatusNotFound, errors.New("Unknown Job Id"))
	}

	ctx := context.Background()
	log := zlog.With().Any("job id", job.Id).Str("url", lo.Substring(job.Url, 0, 15)+"...").Type("downloader", d).Logger()

	err := d.Resume(ctx, job)
	if err != nil {
		log.Err(err).Msg("Failed to resume job")
		return fail(c, http.StatusInternalServerError, err)
	}

	log.Info().Msg("Resumed job")

	return success(c, job)
}

func AllVaultItems(c *echo.Context) error {
	resp := c.Response()
	resp.Header().Set("Content-Type", "text/event-stream")
	resp.Header().Set("Cache-Control", "no-cache")
	resp.Header().Set("Connection", "keep-alive")

	resp.WriteHeader(http.StatusOK)

	ct := http.NewResponseController(resp)
	ct.Flush()

	ticker := time.NewTicker(time.Second * 2)
	defer ticker.Stop()

	for {
		select {
		case <-c.Request().Context().Done():
			return nil
		case <-ticker.C:
			vaultItems, err := gorm.G[models.VaultItem](db.GetDb()).
				Select("id").Order("updated_at DESC").
				Find(c.Request().Context())
			if err != nil {
				return fail(c, http.StatusInternalServerError, err)
			}
			zlog.Debug().Msgf("Found %d vault items", len(vaultItems))

			validVaultIds := mapset.NewSetWithSize[uuid.UUID](0)

			// Fetch remaining from cache
			for i := range len(vaultItems) {
				item, ok := GetCachedVaultItems(vaultItems[i].Id)
				if ok {
					vaultItems[i] = item
					validVaultIds.Add(vaultItems[i].Id)
				}
			}

			go func() {
				// Evict from the cache - sync or async?
				for key := range vaultItemsCache().Keys() {
					if !validVaultIds.Contains(key) {
						vaultItemsCache().Invalidate(key)
					}
				}
			}()

			// Send to stream
			out, err := json.Marshal(vaultItems)
			if err != nil {
				return err
			}

			if _, err := fmt.Fprintf(resp, "data: %s\n\n", out); err != nil {
				return err
			}

			ct.Flush()
		}
	}
}

func DeleteVaultItem(c *echo.Context) error {
	var vaultItem models.VaultItem
	var err error
	if err = c.Bind(&vaultItem); err != nil {
		return fail(c, http.StatusBadRequest, err)
	}
	if vaultItem.Id == uuid.Nil() {
		return fail(c, http.StatusBadRequest, errors.New("vault item id is required"))
	}

	vaultItem, err = gorm.G[models.VaultItem](db.GetDb()).
		Where("id = ?", vaultItem.Id).First(c.Request().Context())
	if err != nil {
		return fail(c, http.StatusNotFound, errors.New("vault item not found"))
	}

	err = db.GetDb().Delete(&vaultItem).Error
	if err != nil {
		return fail(c, http.StatusInternalServerError, err)
	}

	go func(loc string) {
		_, err := backoff.Retry(context.Background(), func() (any, error) {
			err := os.Remove(loc)
			if errors.Is(err, os.ErrNotExist) {
				return nil, nil
			}
			return nil, err
		}, backoff.WithMaxTries(5))
		if err != nil {
			zlog.Error().Err(err).Msg("Failed to remove vault item file")
		}
	}(vaultItem.FilePath)

	// Check if vault item was last in download jobs.
	v, err := gorm.G[models.VaultItem](db.GetDb()).
		Where("download_job_id = ?", vaultItem.DownloadJobId).Count(c.Request().Context(), "id")
	if err == nil && v == 0 {
		// Remove download job files.
		job, err := gorm.G[models.DownloadJob](db.GetDb()).
			Where("id = ? ", vaultItem.DownloadJobId).First(c.Request().Context())
		if err == nil {
			go func(loc string) {
				_, err := backoff.Retry(context.Background(), func() (any, error) {
					err := os.RemoveAll(loc)
					if errors.Is(err, os.ErrNotExist) {
						return nil, nil
					}
					return nil, err
				}, backoff.WithMaxTries(5))
				if err != nil {
					zlog.Error().Err(err).Msg("Failed to remove download job file")
				}
			}(job.Location)
			// Mark job as deleted
			db.GetDb().Table("download_jobs").
				Where("id = ?", job.Id).Update("status", models.DownloadStatusDeleted)
		}
	}

	return success(c, vaultItem)
}
