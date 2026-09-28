package parsers

import (
	"context"
	"time"

	"komorebi-server/configs"
	"komorebi-server/src/dto"

	"github.com/rs/zerolog/log"
	"golang.org/x/sync/errgroup"
)

type Parser interface {
	CanParse(content string) bool
	Parse(content string) dto.ParsedTitle
}

var titleParsers = []Parser{&anitomyParser{}}

func Parse(content string) dto.ParsedTitle {
	v, _ := Cache().ComputeIfAbsent(content, func() (dto.ParsedTitle, bool) {
		for _, parser := range titleParsers {
			if parser.CanParse(content) {
				p := parser.Parse(content)
				return p, false
			}
		}
		return dto.ParsedTitle{}, true
	})
	return v
}

func ParseMany(ctx context.Context, contents []dto.CrawlerResult) {
	start := time.Now()

	if ctx == nil {
		ctx = context.Background()
	}
	g := errgroup.Group{}
	g.SetLimit(100)
	for i, title := range contents {
		g.Go(func() error {
			// Mutex not required as each index is isolated
			contents[i].ParsedTitle = new(Parse(title.Title))
			return nil
		})
	}

	g.Wait()

	if configs.GetConfig().Logger.PrintCrawling {
		log.Debug().
			Dur("duration", time.Since(start)).
			Int("results", len(contents)).
			Msg("Title parsing benchmark")
	}
}
