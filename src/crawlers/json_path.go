package crawlers

import (
	"fmt"
	"strings"

	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/ohler55/ojg/jp"
	"github.com/ohler55/ojg/oj"
	"github.com/rs/zerolog/log"
)

type jsonPathCrawler struct{}

func (c *jsonPathCrawler) Crawl(content []byte, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	logger := log.With().Str("config", config.Key).Type("crawler", c).Logger()
	var dtos []dto.CrawlerResult
	obj, err := oj.Parse(content)
	if err != nil {
		logger.Debug().Err(err).Msg("Failed to parse JSON content")
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

		maxLen := max(len(links), len(titles))

		var pops []any
		if popExpr != nil {
			pops = popExpr.Get(row)
		}
		var sizes []any
		if sizeExpr != nil {
			sizes = sizeExpr.Get(row)
		}

		for i := range maxLen {
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

			t, s := parseTitleAndSize(item.Link)
			if item.Title == "" {
				item.Title = t
			}
			if item.Size == nil || *item.Size == "" {
				item.Size = &s
			}

			dtos = append(dtos, item)
		}
	}

	logger.Debug().Int("count", len(dtos)).Msg("Found results")
	return dtos, nil
}

func getFieldValue(values []any, index int) (string, bool) {
	var val any
	if index < len(values) {
		val = values[index]
	} else if len(values) > 0 {
		val = values[0]
	}
	if val == nil {
		return "", false
	}
	return strings.TrimSpace(fmt.Sprintf("%v", val)), true
}
