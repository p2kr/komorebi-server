package controllers

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"time"
	"uuid"

	"komorebi-server/src/models"

	"komorebi-server/configs"
	"komorebi-server/src/crawlers"
	"komorebi-server/src/downloaders"
	"komorebi-server/src/dto"
	"komorebi-server/src/parsers"

	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
	"github.com/samber/lo"
)

func CrawlerRoutes(g *echo.Group) {
	// Load crawler configs in background
	ctx := context.Background()
	go getCrawlerConfigs(ctx)

	r := g.Group("/crawler")

	r.POST("/search", SearchQuery)
	r.POST("/add", Add)
	r.GET("/active", Active)
	r.POST("/pause", Pause)
	r.DELETE("/delete", Delete)
	r.POST("/resume", Resume)
}

func SearchQuery(c *echo.Context) error {
	var params struct {
		MediaType dto.MediaType
		Query     string
	}

	err := c.Bind(&params)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	ctx := context.Background()
	cfg, ok := getCrawlerConfigs(ctx)
	if !ok || len(cfg) == 0 {
		log.Err(err).Any("configs", cfg).Msg("No config found")
		return fail(c, http.StatusNotFound,
			fmt.Errorf("%s, empty crawler configs", err), "Configs", cfg)
	}

	engine := crawlers.CrawlerEngine{
		Client:    httpClient,
		Query:     params.Query,
		MediaType: params.MediaType,
		Configs:   cfg,
	}

	res, err := engine.Crawl()
	if err != nil {
		log.Err(err).Msg("Failed to crawl")
		return fail(c, http.StatusInternalServerError, err)
	}
	tp := parsers.TitleParser{Ctx: ctx}
	tp.ParseMany(res)

	return success(c, res)
}

type AddPayload struct {
	CrawlerResult dto.CrawlerResult `json:"crawler_result"`
	UserId        uuid.UUID         `json:"user_id"`
}

func Add(c *echo.Context) error {
	var params AddPayload
	if err := c.Bind(&params); err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	// ssrf prevention
	err := validateUrl(params.CrawlerResult.Link)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	log := log.With().Str("url", params.CrawlerResult.Link).Logger()

	d, err := downloaders.GetDownloader(params.CrawlerResult.Link)
	if err != nil {
		log.Err(err).Msg("Failed to get downloader")
		return fail(c, http.StatusBadRequest, err)
	}

	ctx := context.Background()
	job := models.NewDownloadJob(uuid.Nil().String(), configs.GetConfig().Env.VaultLoc,
		params.CrawlerResult.Link, params.CrawlerResult.Title)

	_, err = d.Submit(ctx, &job)
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

	ticker := time.NewTicker(time.Second * 2)
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
		return fail(c, http.StatusNotFound, errors.New("Unknown Job Id"))
	}

	ctx := context.Background()
	log := log.With().Any("job id", job.Id).Str("url", lo.Substring(job.Url, 0, 15)+"...").Type("downloader", d).Logger()
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
	log := log.With().Any("job id", job.Id).Str("url", lo.Substring(job.Url, 0, 15)+"...").Type("downloader", d).Logger()

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
	log := log.With().Any("job id", job.Id).Str("url", lo.Substring(job.Url, 0, 15)+"...").Type("downloader", d).Logger()

	err := d.Resume(ctx, job)
	if err != nil {
		log.Err(err).Msg("Failed to resume job")
		return fail(c, http.StatusInternalServerError, err)
	}

	log.Info().Msg("Resumed job")

	return success(c, job)
}
