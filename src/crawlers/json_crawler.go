package crawlers

import (
	"encoding/json"
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

	dtos = make([]dto.CrawlerResult, len(rows))
	for i := range dtos {
		dtos[i].Source = config.Key
		dtos[i].Category = config.Category
	}

	expr, err = jp.ParseString(config.TitleSelector)
	if err != nil {
		logger.Err(err).Str("selector", truncate(config.TitleSelector)).Msg("Failed to parse JSON title selector")
		return dtos, err
	}

	titles := expr.Get(obj)
	for i, title := range titles {
		if i < len(dtos) {
			dtos[i].Title, _ = title.(string)
			dtos[i].Title = strings.TrimSpace(dtos[i].Title)
		}
	}

	expr, err = jp.ParseString(config.LinkSelector)
	if err != nil {
		logger.Err(err).Str("selector", truncate(config.LinkSelector)).Msg("Failed to parse JSON link selector")
		return dtos, err
	}

	links := expr.Get(obj)
	for i, link := range links {
		if i < len(dtos) {
			dtos[i].Link, _ = link.(string)
			dtos[i].Link = strings.TrimSpace(dtos[i].Link)
		}
	}

	if config.PopularitySelector != nil {
		expr, err = jp.ParseString(*config.PopularitySelector)
		if err == nil {
			pops := expr.Get(obj)
			for i, pop := range pops {
				if i < len(dtos) {
					pop2, _ := pop.(string)
					pop2 = strings.TrimSpace(pop2)
					dtos[i].Popularity = &pop2
				}
			}
		}
	}

	if config.SizeSelector != nil {
		expr, err = jp.ParseString(*config.SizeSelector)
		if err == nil {
			sizes := expr.Get(obj)
			for i, size := range sizes {
				if i < len(dtos) {
					size2, _ := size.(string)
					size2 = strings.TrimSpace(size2)
					dtos[i].Size = &size2
				}
			}
		}
	}

	// Filter out invalids.
	validDtos := lo.Filter(dtos, func(dto dto.CrawlerResult, _ int) bool {
		errs := models.ICrawlerResult.Validate(&dto)
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
