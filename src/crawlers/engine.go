package crawlers

import (
	"context"
	"errors"
	"sync"
	"time"

	"komorebi-server/configs"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/rs/zerolog/log"
	"golang.org/x/sync/errgroup"
	"resty.dev/v3"
)

type Crawler interface {
	Crawl(content []byte, config *models.CrawlerConfig) ([]dto.CrawlerResult, error)
}

type CrawlerEngine struct {
	Client    *resty.Client
	Query     string
	MediaType dto.MediaType
	Configs   []models.CrawlerConfig
	Ctx       context.Context
}

var crawlers = []Crawler{&jsonCrawler{}, &htmlCrawler{}}

func (c *CrawlerEngine) Crawl() ([]dto.CrawlerResult, error) {
	results := make([][]dto.CrawlerResult, len(c.Configs))

	if c.Ctx == nil {
		c.Ctx = context.Background()
	}

	g := errgroup.Group{}

	g.SetLimit(10)

	dur, err := time.ParseDuration(configs.GetConfig().HttpClient.CrawlerTimeout)
	if err != nil {
		dur = time.Second * 60
	}

	for idx, config := range c.Configs {
		if config.IsDeleted {
			continue
		}
		g.Go(func() error {
			rCtx, cancel := context.WithTimeout(c.Ctx, dur)
			defer cancel()

			rDtos, err := c.CrawlConfig(rCtx, &config)
			if len(rDtos) == 0 && err != nil {
				return err
			}

			results[idx] = rDtos
			return nil
		})
	}

	err = g.Wait()

	var dtos []dto.CrawlerResult

	// Flatten and validate
	for _, batch := range results {
		for _, d := range batch {
			errs := dto.ICrawlerResult.Validate(&d)
			if errs != nil {
				continue
			}
			dtos = append(dtos, d)
		}
	}

	log.Debug().Err(err).Msg("crawler engine completed")

	if len(dtos) == 0 && err != nil {
		return dtos, err
	}

	return dtos, nil
}

func (c *CrawlerEngine) fetchHtml(ctx context.Context, config *models.CrawlerConfig) ([]byte, error) {
	u, err := ReplaceInURL(config.Url, map[string]string{
		"{query}": c.Query,
	})
	if err != nil {
		log.Err(err).Str("base url", config.Url).Str("query", c.Query).Msg("Invalid URL")
		return nil, err
	}

	resp, err := c.Client.R().SetContext(ctx).Get(u)
	if err != nil {
		log.Err(err).Str("url", truncate(u)).Str("config", truncate(config.Key)).Msg("Failed to get [url]")
		return nil, err
	}
	if resp.StatusCode() != 200 {
		log.Error().Str("url", truncate(u)).Str("config", truncate(config.Key)).Int("status_code", resp.StatusCode()).Str("status", truncate(resp.Status())).Msg("Failed to get [url]")
		return nil, errors.New("response status " + resp.Status())
	}

	return resp.Bytes(), nil
}

func (c *CrawlerEngine) CrawlConfig(
	ctx context.Context,
	config *models.CrawlerConfig,
) ([]dto.CrawlerResult, error) {
	start := time.Now()
	var result []dto.CrawlerResult

	logger := log.With().
		Str("query", truncate(c.Query)).
		Str("config", truncate(config.Key)).
		Logger()

	content, err := c.fetchHtml(ctx, config)
	if err != nil {
		logger.Error().Err(err).Msg("fetch html err")
		return result, err
	}

	g := errgroup.Group{}
	g.SetLimit(100)

	mu := sync.Mutex{}

	for _, crawler := range crawlers {
		g.Go(func() error {
			resDto, err := crawler.Crawl(content, config)
			log := logger.With().Type("crawler", crawler).Logger()
			if err != nil {
				log.Warn().Err(err).Msg("crawling failed")
				return err
			}
			mu.Lock()
			result = append(result, resDto...)
			mu.Unlock()
			log.Info().
				Int("results", len(resDto)).
				Str("duration", time.Since(start).Round(time.Millisecond).String()).
				Msg("Crawling Info")
			return nil
		})
	}

	err = g.Wait()

	if len(result) == 0 && err != nil {
		logger.Err(err).Msg("Failed to crawl")
		return result, err
	}

	return result, nil
}
