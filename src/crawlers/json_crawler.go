package crawlers

import (
	"fmt"
	"net/url"
	"strings"

	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/ohler55/ojg/jp"
	"github.com/ohler55/ojg/oj"
	"github.com/rs/zerolog/log"
)

type jsonCrawler struct{}

func (c *jsonCrawler) Crawl(content []byte, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	logger := log.With().Str("config", config.Key).Type("crawler", c).Logger()
	var dtos []dto.CrawlerResult
	obj, err := oj.Parse(content)
	if err != nil {
		logger.Warn().Err(err).Msg("Failed to parse JSON content")
		return dtos, err
	}

	expr, ok := getJpExpr(config.RowSelector)
	if !ok {
		logger.Error().Str("selector", truncate(config.RowSelector)).Msg("Failed to parse JSON row selector")
		return dtos, err
	}

	rows := expr.Get(obj)

	var titleExpr jp.Expr
	if config.TitleSelector != "" {
		titleExpr, ok = getJpExpr(config.TitleSelector)
		if !ok {
			logger.Error().Str("selector", truncate(config.TitleSelector)).Msg("Failed to parse JSON title selector")
			return dtos, err
		}
	}

	linkExpr, ok := getJpExpr(config.LinkSelector)
	if !ok {
		logger.Err(err).Str("selector", truncate(config.LinkSelector)).Msg("Failed to parse JSON link selector")
		return dtos, err
	}

	var popExpr, sizeExpr jp.Expr
	if config.PopularitySelector != nil {
		popExpr, _ = getJpExpr(*config.PopularitySelector)
	}
	if config.SizeSelector != nil {
		sizeExpr, _ = getJpExpr(*config.SizeSelector)
	}

	for _, row := range rows {
		var titles []any
		if titleExpr != nil {
			titles = titleExpr.Get(row)
		}

		links := linkExpr.Get(row)
		if len(links) == 0 {
			continue
		}

		maxLen := len(titles)
		if len(links) > maxLen {
			maxLen = len(links)
		}

		var pops []any
		if popExpr != nil {
			pops = popExpr.Get(row)
		}
		var sizes []any
		if sizeExpr != nil {
			sizes = sizeExpr.Get(row)
		}

		for i := 0; i < maxLen; i++ {
			item := dto.CrawlerResult{
				Source:   config.Key,
				Category: config.Category,
			}

			// Populate title from selector (with index fallback to first element).
			if val, ok := getFieldValue(titles, i); ok {
				item.Title = val
			}

			// Populate link (with index fallback to first element).
			if val, ok := getFieldValue(links, i); ok {
				item.Link = val
			}

			// Populate optional fields.
			if val, ok := getFieldValue(pops, i); ok {
				item.Popularity = &val
			}

			if val, ok := getFieldValue(sizes, i); ok {
				item.Size = &val
			}

			// If title is still empty and the link is a magnet URI, derive
			// title (and optionally size) from the magnet's own metadata.
			if item.Title == "" && strings.HasPrefix(item.Link, "magnet:") {
				if u, err := url.Parse(item.Link); err == nil {
					q := u.Query()
					if dn := q.Get("dn"); dn != "" {
						item.Title = dn
					}
					if item.Size == nil {
						if xl := q.Get("xl"); xl != "" {
							item.Size = &xl
						}
					}
				}
			}

			dtos = append(dtos, item)
		}
	}

	logger.Debug().Int("count", len(dtos)).Msg("Found results")
	return dtos, nil
}

func getFieldValue(values []any, index int) (string, bool) {
	if index < len(values) {
		return strings.TrimSpace(fmt.Sprintf("%v", values[index])), true
	} else if len(values) > 0 {
		return strings.TrimSpace(fmt.Sprintf("%v", values[0])), true
	}
	return "", false
}
