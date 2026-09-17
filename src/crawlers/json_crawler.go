package crawlers

import (
	"encoding/json"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"strings"

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
	v, notfound := jsonCrawlerCache.GetIfPresent(content)
	canCrawl := v
	if notfound {
		var res map[string]any
		err := json.Unmarshal([]byte(content), &res)
		if err != nil {
			canCrawl = false
		} else {
			canCrawl = true
		}
		jsonCrawlerCache.Set(content, canCrawl)
	}
	return canCrawl
}

func (c *jsonCrawler) Crawl(content string, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	var dtos []dto.CrawlerResult
	obj, err := oj.ParseString(content)
	if err != nil {
		return dtos, err
	}

	expr, err := jp.ParseString(config.RowSelector)
	if err != nil {
		return dtos, err
	}

	rows := expr.Get(obj)

	dtos = make([]dto.CrawlerResult, len(rows))

	expr, err = jp.ParseString(config.TitleSelector)
	if err != nil {
		return dtos, err
	}

	titles := expr.Get(obj)
	for i, title := range titles {
		dtos[i].Title, _ = title.(string)
		dtos[i].Title = strings.TrimSpace(dtos[i].Title)
	}

	expr, err = jp.ParseString(config.LinkSelector)
	if err != nil {
		return dtos, err
	}

	links := expr.Get(obj)
	for i, link := range links {
		dtos[i].Link, _ = link.(string)
		dtos[i].Link = strings.TrimSpace(dtos[i].Link)
	}

	expr, err = jp.ParseString(*config.PopularitySelector)
	if err == nil {
		pops := expr.Get(obj)
		for i, pop := range pops {
			pop2, _ := pop.(string)
			pop2 = strings.TrimSpace(pop2)
			dtos[i].Popularity = &pop2
		}
	}

	expr, err = jp.ParseString(*config.SizeSelector)
	if err == nil {
		sizes := expr.Get(obj)
		for i, size := range sizes {
			size2, _ := size.(string)
			size2 = strings.TrimSpace(size2)
			dtos[i].Popularity = &size2
		}
	}

	// Filter out invalids.
	validDtos := lo.Filter(dtos, func(dto dto.CrawlerResult, _ int) bool {
		errs := models.ICrawlerConfig.Validate(&dto)
		if errs != nil {
			log.Debug().Any("errors", zog.Issues.Flatten(errs)).Any("config", config.Id).Any("dto", dto).Msg("invalid dto")
			return false
		} else {
			return true
		}
	})

	return validDtos, nil
}
