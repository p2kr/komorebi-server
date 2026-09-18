package crawlers

import (
	"encoding/json"
	"net/url"
	"strings"

	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/Oudwins/zog"
	"github.com/maypok86/otter/v2"
	"github.com/ohler55/ojg/jp"
	"github.com/ohler55/ojg/oj"
	"github.com/rs/zerolog/log"
	"github.com/samber/lo"
)

type jsonCrawler struct{}

var jsonCrawlerCache, _ = otter.New[string, bool](&canCrawlCacheConfig)

func (c *jsonCrawler) CanCrawl(content string) bool {
	v, ok := jsonCrawlerCache.ComputeIfAbsent(content, func() (bool, bool) {
		trimmed := strings.TrimSpace(content)
		if !strings.HasPrefix(trimmed, "{") && !strings.HasPrefix(trimmed, "[") {
			return false, false
		}
		var res any
		err := json.Unmarshal([]byte(trimmed), &res)
		if err != nil {
			return false, false
		}
		return true, false
	})
	return v && ok
}

func (c *jsonCrawler) Crawl(content string, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	logger := log.With().Str("config", config.Key).Type("crawler", c).Logger()
	var dtos []dto.CrawlerResult
	obj, err := oj.ParseString(content)
	if err != nil {
		logger.Err(err).Msg("Failed to parse JSON content")
		return dtos, err
	}

	expr, err := jp.ParseString(config.RowSelector)
	if err != nil {
		logger.Err(err).Str("selector", truncate(config.RowSelector)).Msg("Failed to parse JSON row selector")
		return dtos, err
	}

	rows := expr.Get(obj)

	var titleExpr jp.Expr
	if config.TitleSelector != "" {
		titleExpr, err = jp.ParseString(config.TitleSelector)
		if err != nil {
			logger.Err(err).Str("selector", truncate(config.TitleSelector)).Msg("Failed to parse JSON title selector")
			return dtos, err
		}
	}

	linkExpr, err := jp.ParseString(config.LinkSelector)
	if err != nil {
		logger.Err(err).Str("selector", truncate(config.LinkSelector)).Msg("Failed to parse JSON link selector")
		return dtos, err
	}

	var popExpr, sizeExpr jp.Expr
	if config.PopularitySelector != nil {
		popExpr, _ = jp.ParseString(*config.PopularitySelector)
	}
	if config.SizeSelector != nil {
		sizeExpr, _ = jp.ParseString(*config.SizeSelector)
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
			if i < len(titles) {
				if raw, ok := titles[i].(string); ok {
					item.Title = strings.TrimSpace(raw)
				}
			} else if len(titles) > 0 {
				if raw, ok := titles[0].(string); ok {
					item.Title = strings.TrimSpace(raw)
				}
			}

			// Populate link (with index fallback to first element).
			if i < len(links) {
				item.Link, _ = links[i].(string)
				item.Link = strings.TrimSpace(item.Link)
			} else if len(links) > 0 {
				item.Link, _ = links[0].(string)
				item.Link = strings.TrimSpace(item.Link)
			}

			// Populate optional fields.
			if i < len(pops) {
				if pop2, ok := pops[i].(string); ok {
					pop2 = strings.TrimSpace(pop2)
					item.Popularity = &pop2
				}
			} else if len(pops) > 0 {
				if pop2, ok := pops[0].(string); ok {
					pop2 = strings.TrimSpace(pop2)
					item.Popularity = &pop2
				}
			}

			if i < len(sizes) {
				if size2, ok := sizes[i].(string); ok {
					size2 = strings.TrimSpace(size2)
					item.Size = &size2
				}
			} else if len(sizes) > 0 {
				if size2, ok := sizes[0].(string); ok {
					size2 = strings.TrimSpace(size2)
					item.Size = &size2
				}
			}

			// If title is still empty and the link is a magnet URI, derive
			// title (and optionally size) from the magnet's own metadata.
			if item.Title == "" && strings.HasPrefix(item.Link, "magnet:") {
				if u, err := url.Parse(item.Link); err == nil {
					if dn := u.Query().Get("dn"); dn != "" {
						item.Title = dn
					}
					if item.Size == nil {
						if xl := u.Query().Get("xl"); xl != "" {
							item.Size = &xl
						}
					}
				}
			}

			dtos = append(dtos, item)
		}
	}

	// Filter out invalids.
	validDtos := lo.Filter(dtos, func(d dto.CrawlerResult, _ int) bool {
		errs := dto.ICrawlerResult.Validate(&d)
		if errs != nil {
			logger.Warn().Any("errors", zog.Issues.Flatten(errs)).Msg("invalid dto")
			return false
		} else {
			return true
		}
	})

	logger.Debug().Int("count", len(validDtos)).Msg("Found results")
	return validDtos, nil
}
