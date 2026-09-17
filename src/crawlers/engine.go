package crawlers

import (
	"errors"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"strings"

	"github.com/rs/zerolog/log"
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
}

var crawlers = []Crawler{&htmlCrawler{}, &jsonCrawler{}}

func (c *CrawlerEngine) Crawl() ([]dto.CrawlerResult, error) {
	var dtos []dto.CrawlerResult
	var errs []error

	for _, config := range *c.Configs {
		content, err := c.FetchHtml(&config)
		if err == nil {
			for _, crawler := range crawlers {
				if crawler.CanCrawl(content) {
					dto, err := crawler.Crawl(content, &config)
					if err == nil {
						// add to list
						dtos = append(dtos, dto...)
						break
					} else {
						errs = append(errs, err)
					}
				}
			}
		} else {
			errs = append(errs, err)
		}
	}

	if len(dtos) == 0 && len(errs) > 0 {
		return dtos, errors.Join(errs...)
	}

	return dtos, nil
}

func (c *CrawlerEngine) FetchHtml(config *models.CrawlerConfig) (string, error) {
	url := strings.ReplaceAll(config.Url, "{query}", c.Query)

	resp, err := c.Client.R().Get(url)
	if err != nil || resp.StatusCode() == 200 {
		log.Err(err).Str("url", url).Str("status", resp.Status()).Msg("Failed to get [url]")
		return "", errors.Join(err, errors.New("response status "+resp.Status()))
	}

	return resp.String(), nil
}
