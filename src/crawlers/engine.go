package crawlers

import (
	"errors"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"net/url"
	"strings"

	"github.com/rs/zerolog/log"
	"resty.dev/v3"
)

type Crawler interface {
	Name() string
	CanCrawl(content string) bool
	Crawl(content string, config *models.CrawlerConfig) ([]dto.CrawlerResult, error)
}

type CrawlerEngine struct {
	Client    *resty.Client
	Query     string
	MediaType dto.MediaType
	Configs   *[]models.CrawlerConfig
}

var crawlers = []Crawler{&jsonCrawler{}, &htmlCrawler{}}

func (c *CrawlerEngine) Crawl() ([]dto.CrawlerResult, error) {
	var dtos []dto.CrawlerResult
	var errs []error

	for _, config := range *c.Configs {
		if config.IsDeleted {
			continue
		}

		l := log.Info().Str("query", truncate(c.Query)).Str("config", truncate(config.Key))

		content, err := c.FetchHtml(&config)
		if err == nil {
			for _, crawler := range crawlers {
				if crawler.CanCrawl(content) {
					l.Str("crawler", truncate(crawler.Name()))
					dto, err := crawler.Crawl(content, &config)
					if err == nil {
						// add to list
						dtos = append(dtos, dto...)
						l.Int("results", len(dto))
						break
					} else {
						l.AnErr("error in can crawl", err)
						errs = append(errs, err)
					}
				}
			}
		} else {
			l.AnErr("fetch html err", err)
			errs = append(errs, err)
		}

		l.Msg("Crawling Info")
	}

	if len(dtos) == 0 && len(errs) > 0 {
		return dtos, errors.Join(errs...)
	}

	return dtos, nil
}

func (c *CrawlerEngine) FetchHtml(config *models.CrawlerConfig) (string, error) {
	u := strings.ReplaceAll(config.Url, "{query}", url.PathEscape(c.Query))
	resp, err := c.Client.R().Get(u)
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
