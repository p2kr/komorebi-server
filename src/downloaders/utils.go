package downloaders

import (
	"komorebi-server/configs"

	"github.com/cenkalti/log"
	"github.com/rs/zerolog"
	zlog "github.com/rs/zerolog/log"
)

type customTorrentLogger struct{}

func (c customTorrentLogger) SetFormatter(formatter log.Formatter) {
	// Do nothing
}

func (c customTorrentLogger) SetLevel(level log.Level) {
	// Do nothing
}

func (c customTorrentLogger) Handle(record *log.Record) {
	// Do nothing
	zlogLevel := zerolog.WarnLevel
	switch record.Level {
	case 0, 1:
		zlogLevel = zerolog.ErrorLevel
	case 2:
		zlogLevel = zerolog.WarnLevel
	default:
		if configs.GetConfig().TorrentClient.Debug {
			zlogLevel = zerolog.DebugLevel
		} else {
			return
		}
	}
	zlog.WithLevel(zlogLevel).
		Int("line", record.Line).
		Str("filename", record.Filename).
		Msgf("%s", record.Message)
}

func (c customTorrentLogger) Close() error {
	return nil
}
