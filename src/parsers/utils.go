package parsers

import (
	"sync"

	"komorebi-server/src/dto"

	"github.com/maypok86/otter/v2"
	zlog "github.com/rs/zerolog/log"
)

var Cache = sync.OnceValue(func() *otter.Cache[string, dto.ParsedTitle] {
	c, err := otter.New(&otter.Options[string, dto.ParsedTitle]{
		MaximumSize: 1000,
	})
	if err != nil {
		zlog.Err(err).Msg("error creating new otter")
		return nil
	}
	return c
})
