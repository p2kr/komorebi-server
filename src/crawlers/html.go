package crawlers

import (
	"bytes"
	"net/url"
	"strings"

	"komorebi-server/configs"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/PuerkitoBio/goquery"
	"github.com/rs/zerolog/log"
)

type htmlCrawler struct{}

func (c *htmlCrawler) Crawl(content []byte, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	logger := log.With().Str("config", config.Key).Type("crawler", c).Logger()
	var dtos []dto.CrawlerResult
	reader := bytes.NewReader(content)
	doc, err := goquery.NewDocumentFromReader(reader)
	if err != nil {
		logger.Debug().Err(err).Msg("Failed to parse HTML document")
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
		title := strings.TrimSpace(findWithMatcher(s, config.TitleSelector).Text())
		link, _ := findWithMatcher(s, config.LinkSelector).Attr("href")
		link = strings.TrimSpace(link)

		var popularity, size string
		if config.PopularitySelector != nil {
			popularity = strings.TrimSpace(findWithMatcher(s, *config.PopularitySelector).Text())
		}
		if config.SizeSelector != nil {
			size = strings.TrimSpace(findWithMatcher(s, *config.SizeSelector).Text())
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
