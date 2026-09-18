package crawlers

import (
	"net/url"
	"strings"

	"komorebi-server/configs"
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

	printCrawling := false
	if cfg := configs.GetConfig(); cfg != nil {
		printCrawling = cfg.Logger.PrintCrawling
	}

	doc.Find(config.RowSelector).Each(func(i int, s *goquery.Selection) {
		title := strings.TrimSpace(s.Find(config.TitleSelector).Text())
		link, _ := s.Find(config.LinkSelector).Attr("href")
		link = strings.TrimSpace(link)

		var popularity, size string
		if config.PopularitySelector != nil {
			popularity = strings.TrimSpace(s.Find(*config.PopularitySelector).Text())
		}
		if config.SizeSelector != nil {
			size = strings.TrimSpace(s.Find(*config.SizeSelector).Text())
		}

		if title == "" && strings.HasPrefix(link, "magnet:") {
			if u, err := url.Parse(link); err == nil {
				if dn := u.Query().Get("dn"); dn != "" {
					title = dn
				}
				if size == "" {
					if xl := u.Query().Get("xl"); xl != "" {
						size = xl
					}
				}
			}
		}

		if title == "" {
			if printCrawling {
				logger.Debug().Int("index", i).Str("title", truncate(title)).Msg("Skipping empty title")
			}
			return
		}

		_, err := url.ParseRequestURI(link)
		if err != nil {
			if printCrawling {
				logger.Debug().Int("index", i).Str("title", truncate(title)).Str("link", truncate(link)).AnErr("link parsing failed", err).Msg("Failed to parse link")
			}
			return
		}

		dto := dto.CrawlerResult{
			Title:    title,
			Link:     link,
			Source:   config.Key,
			Category: config.Category,
		}

		if popularity != "" {
			dto.Popularity = &popularity
		}
		if size != "" {
			dto.Size = &size
		}

		if printCrawling {
			logger.Debug().
				Int("index", i).
				Str("title", truncate(title)).
				Str("link", truncate(link)).
				Str("popularity", truncate(popularity)).
				Str("size", truncate(size)).
				Str("category", truncate(string(config.Category))).
				Msg("Found result")
		}

		dtos = append(dtos, dto)
	})

	logger.Debug().Int("results", len(dtos)).Msg("Crawling Ended")

	return dtos, nil
}
