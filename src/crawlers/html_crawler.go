package crawlers

import (
	"net/url"
	"strings"

	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/PuerkitoBio/goquery"
	"github.com/maypok86/otter/v2"
	"github.com/rs/zerolog/log"
)

type htmlCrawler struct{}

var htmlCrawlerCache, _ = otter.New[string, bool](&canCrawlCacheConfig)

func (c *htmlCrawler) CanCrawl(content string) bool {
	v, ok := htmlCrawlerCache.ComputeIfAbsent(content, func() (bool, bool) {
		reader := strings.NewReader(content)
		_, err := goquery.NewDocumentFromReader(reader)
		if err != nil {
			return false, false
		}
		return true, false
	})
	return v && ok
}

func (c *htmlCrawler) Crawl(content string, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	logger := log.With().Str("config", config.Key).Type("crawler", c).Logger()
	var dtos []dto.CrawlerResult
	reader := strings.NewReader(content)
	doc, err := goquery.NewDocumentFromReader(reader)
	if err != nil {
		logger.Err(err).Msg("Failed to parse document")
		return dtos, err
	} else {
		logger.Info().Msg("Parsed document")
	}

	logger.Debug().Msg("Crawling started")
	defer logger.Debug().Int("results", len(dtos)).Msg("Crawling Ended")

	doc.Find(config.RowSelector).Each(func(i int, s *goquery.Selection) {
		l := logger.Debug().Int("index", i)

		dto := dto.CrawlerResult{}

		title := strings.TrimSpace(s.Find(config.TitleSelector).Text())
		l.Str("title", truncate(title))
		if title == "" {
			l.Msg("Skipping empty title")
			return
		}
		dto.Title = title

		link, _ := s.Find(config.LinkSelector).Attr("href")
		l.Str("link", truncate(link))
		// Check if link is valid url
		_, err := url.ParseRequestURI(link)
		if err != nil {
			l.AnErr("link parsing failed", err).Msg("Failed to parse link")
			return
		}
		dto.Link = link

		if config.PopularitySelector != nil {
			popularity := strings.TrimSpace(s.Find(*config.PopularitySelector).Text())
			l.Str("popularity", truncate(popularity))
			if popularity != "" {
				dto.Popularity = &popularity
			}
		}

		if config.SizeSelector != nil {
			size := strings.TrimSpace(s.Find(*config.SizeSelector).Text())
			l.Str("size", truncate(size))
			if size != "" {
				dto.Size = &size
			}
		}
		l.Str("category", truncate(string(config.Category)))
		dto.Source = config.Key
		dto.Category = config.Category
		l.Msg("Found result")
		dtos = append(dtos, dto)
	})
	return dtos, nil
}
