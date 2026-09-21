package crawlers

import (
	"strings"

	"komorebi-server/src/dto"
	"komorebi-server/src/models"

	"github.com/rs/zerolog/log"
	"github.com/tidwall/gjson"
)

type gjsonCrawler struct{}

func (c *gjsonCrawler) Crawl(content []byte, config *models.CrawlerConfig) ([]dto.CrawlerResult, error) {
	logger := log.With().Str("config", config.Key).Type("crawler", c).Logger()
	var dtos []dto.CrawlerResult

	// 1. Silent HTML Rejector: Fixes the log spam bug!
	if !gjson.ValidBytes(content) {
		return dtos, nil
	}

	// 2. Syntax Bridge: Converts JSONPath to GJSON syntax seamlessly
	sanitizePath := func(path string) string {
		p := strings.TrimSpace(path)
		if p == "$" {
			return ""
		} // GJSON root
		p = strings.ReplaceAll(p, "$..", "..") // Deep search
		p = strings.TrimPrefix(p, "$.")
		p = strings.ReplaceAll(p, "[*]", "") // JSONPath array brackets
		return p
	}

	// 3. Array Bridge: Forces single items into slices to mimic json_path behavior
	getAsArray := func(res gjson.Result) []gjson.Result {
		if !res.Exists() {
			return nil
		}
		if res.IsArray() {
			return res.Array()
		}
		return []gjson.Result{res}
	}

	// Fetch base rows
	rowRes := gjson.GetBytes(content, sanitizePath(config.RowSelector))
	rows := getAsArray(rowRes)

	for _, row := range rows {
		var titles, links, pops, sizes []gjson.Result

		if config.TitleSelector != "" {
			titles = getAsArray(row.Get(sanitizePath(config.TitleSelector)))
		}
		links = getAsArray(row.Get(sanitizePath(config.LinkSelector)))

		if len(links) == 0 {
			continue
		}

		if config.PopularitySelector != nil && *config.PopularitySelector != "" {
			pops = getAsArray(row.Get(sanitizePath(*config.PopularitySelector)))
		}
		if config.SizeSelector != nil && *config.SizeSelector != "" {
			sizes = getAsArray(row.Get(sanitizePath(*config.SizeSelector)))
		}

		// 4. The Zipping Engine: Restores functionality for SubsPlease / Nyaa
		maxLen := max(len(links), len(titles))

		for i := range maxLen {
			item := dto.CrawlerResult{
				Source:   config.Key,
				Category: config.Category,
			}

			if i < len(titles) {
				item.Title = strings.TrimSpace(titles[i].String())
			} else if len(titles) > 0 {
				item.Title = strings.TrimSpace(titles[0].String())
			}

			if i < len(links) {
				item.Link = strings.TrimSpace(links[i].String())
			} else if len(links) > 0 {
				item.Link = strings.TrimSpace(links[0].String())
			}

			if i < len(pops) {
				pop := strings.TrimSpace(pops[i].String())
				if pop != "" {
					item.Popularity = &pop
				}
			} else if len(pops) > 0 {
				pop := strings.TrimSpace(pops[0].String())
				if pop != "" {
					item.Popularity = &pop
				}
			}

			if i < len(sizes) {
				size := strings.TrimSpace(sizes[i].String())
				if size != "" {
					item.Size = &size
				}
			} else if len(sizes) > 0 {
				size := strings.TrimSpace(sizes[0].String())
				if size != "" {
					item.Size = &size
				}
			}

			// Magnet Fallback
			t, s := parseTitleAndSize(item.Link)
			if item.Title == "" {
				item.Title = t
			}
			if item.Size == nil || *item.Size == "" {
				if s != "" {
					item.Size = &s
				}
			}

			dtos = append(dtos, item)
		}
	}

	logger.Debug().Int("count", len(dtos)).Msg("Found results")
	return dtos, nil
}
