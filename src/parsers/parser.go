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

type TitleParser struct {
	Ctx context.Context
}

var titleParsers = []Parser{&anitomyParser{}}

func (p *TitleParser) Parse(content string) *dto.ParsedTitle {
	for _, parser := range titleParsers {
		if parser.CanParse(content) {
			p := parser.Parse(content)
			return &p
		}
	}
	return nil
}

func (p *TitleParser) ParseMany(contents []dto.CrawlerResult) {
	start := time.Now()

	if p.Ctx == nil {
		p.Ctx = context.Background()
	}
	g := errgroup.Group{}
	g.SetLimit(100)
	for i, title := range contents {
		g.Go(func() error {
			// Mutex not required as each index is isolated
			contents[i].ParsedTitle = p.Parse(title.Title)
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
