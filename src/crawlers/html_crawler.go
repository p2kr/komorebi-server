package crawlers

import (
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"net/url"
	"strings"

	"github.com/PuerkitoBio/goquery"
	"github.com/maypok86/otter/v2"
)

type htmlCrawler struct{}

var htmlCrawlerCache, _ = otter.New[string, bool](&canCrawlCacheConfig)

func (c *htmlCrawler) CanCrawl(content string) bool {
	v, notfound := htmlCrawlerCache.GetIfPresent(content)
	canCrawl := v
	if notfound {
		reader := strings.NewReader(content)
		_, err := goquery.NewDocumentFromReader(reader)
		if err != nil {
			canCrawl = false
		} else {
			canCrawl = true
		}
		htmlCrawlerCache.Set(content, canCrawl)
	}
	return canCrawl
}

func (c *htmlCrawler) Crawl(content string, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	var dtos []dto.CrawlerResult
	reader := strings.NewReader(content)
	doc, err := goquery.NewDocumentFromReader(reader)
	if err != nil {
		return dtos, err
	}

	doc.Find(config.RowSelector).Each(func(i int, s *goquery.Selection) {
		dto := dto.CrawlerResult{}

		title := strings.TrimSpace(s.Find(config.TitleSelector).Text())
		if title == "" {
			return
		}

		link, _ := s.Find(config.LinkSelector).Attr("href")
		// Check if link is valid url
		_, err := url.ParseRequestURI(link)
		if err != nil {
			return
		}

		popularity := strings.TrimSpace(s.Find(*config.PopularitySelector).Text())
		if popularity != "" {
			dto.Popularity = &popularity
		}

		size := strings.TrimSpace(s.Find(*config.SizeSelector).Text())
		if size != "" {
			dto.Size = &size
		}

		dtos = append(dtos, dto)
	})
	return dtos, nil
}
