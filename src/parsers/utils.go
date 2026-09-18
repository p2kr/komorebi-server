package parsers

import (
	"komorebi-server/src/dto"

	"github.com/maypok86/otter/v2"
)

var parsersCache = otter.Options[string, dto.ParsedTitle]{
	MaximumSize: 100,
}
