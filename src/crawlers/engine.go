package crawlers

import (
	"context"
	"errors"
	"net/url"
	"strings"
	"sync"
	"time"

	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/rs/zerolog/log"
	"golang.org/x/sync/semaphore"
	"resty.dev/v3"
)

type Crawler interface {
	CanCrawl(content string) bool
	Crawl(content string, config *models.CrawlerConfig) ([]dto.CrawlerResult, error)
}

type CrawlerEngine struct {
	Client    *resty.Client
	Query     string
	MediaType dto.MediaType
	Configs   *[]models.CrawlerConfig
	Ctx       context.Context
}

var crawlers = []Crawler{&jsonCrawler{}, &htmlCrawler{}}

func (c *CrawlerEngine) Crawl() ([]dto.CrawlerResult, error) {
	mu := sync.Mutex{}
	wg := sync.WaitGroup{}           // To wait for all goroutines
	sem := semaphore.NewWeighted(10) // semaphore to limit max goroutines

	dtos := make([]dto.CrawlerResult, 0, len(*c.Configs))
	errs := make([]error, 0, len(*c.Configs))

	if c.Ctx == nil {
		c.Ctx = context.Background()
	}

	for _, config := range *c.Configs {
		if config.IsDeleted {
			continue
		}

		if err := sem.Acquire(c.Ctx, 1); err != nil {
			mu.Lock()
			errs = append(errs, err)
			mu.Unlock()
			continue
		}
		wg.Add(1)
		go func() {
			defer sem.Release(1)
			defer wg.Done()

			rCtx, cancel := context.WithTimeout(c.Ctx, time.Second*60)
			defer cancel()

			rDtos, rErrs := c.CrawlConfig(rCtx, &config)
			mu.Lock()
			dtos = append(dtos, rDtos...)
			errs = append(errs, rErrs...)
			mu.Unlock()
		}()

	}

	wg.Wait()

	if len(dtos) == 0 && len(errs) > 0 {
		return dtos, errors.Join(errs...)
	}

	return dtos, nil
}

func (c *CrawlerEngine) fetchHtml(ctx context.Context, config *models.CrawlerConfig) (string, error) {
	u := strings.ReplaceAll(config.Url, "{query}", url.PathEscape(c.Query))
	resp, err := c.Client.R().SetContext(ctx).Get(u)
	if err != nil {
		log.Err(err).Str("url", truncate(u)).Str("config", truncate(config.Key)).Msg("Failed to get [url]")
		return "", err
	}
	if resp.StatusCode() != 200 {
		log.Error().Str("url", truncate(u)).Str("config", truncate(config.Key)).Int("status_code", resp.StatusCode()).Str("status", truncate(resp.Status())).Msg("Failed to get [url]")
		return "", errors.New("response status " + resp.Status())
	}

	return resp.String(), nil
}

func (c *CrawlerEngine) CrawlConfig(
	ctx context.Context,
	config *models.CrawlerConfig,
) ([]dto.CrawlerResult, []error) {
	var dtos []dto.CrawlerResult
	var errs []error

	logger := log.With().
		Str("query", truncate(c.Query)).
		Str("config", truncate(config.Key)).
		Logger()

	content, err := c.fetchHtml(ctx, config)
	if err != nil {
		logger.Error().Err(err).Msg("fetch html err")
		errs = append(errs, err)
		return dtos, errs
	}

	for _, crawler := range crawlers {
		if crawler.CanCrawl(content) {
			crawlLog := logger.With().Type("crawler", crawler).Logger()

			resDto, err := crawler.Crawl(content, config)
			if err == nil {
				dtos = resDto
				crawlLog.Info().Int("results", len(resDto)).Msg("Crawling Info")
				break
			} else {
				crawlLog.Error().Err(err).Msg("error in can crawl")
				errs = append(errs, err)
			}
		}
	}

	return dtos, errs
}
