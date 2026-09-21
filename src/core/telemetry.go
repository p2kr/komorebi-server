package core

import (
	"log/slog"
	"os"
	"strings"

	"komorebi-server/configs"

	"github.com/rs/zerolog"
	zlog "github.com/rs/zerolog/log"
)

func SetupLogger(config *configs.Config) {
	// Setup log level
	logLevel := config.Logger.LogLevel

	if strings.TrimSpace(logLevel) == "" {
		logLevel = "debug"
	}

	lvl, err := zerolog.ParseLevel(logLevel)
	if err != nil {
		lvl = zerolog.DebugLevel
	}
	zerolog.SetGlobalLevel(lvl)

	// Setup logger
	if config.Logger.Pretty {
		zlog.Logger = zlog.Output(zerolog.ConsoleWriter{Out: os.Stderr})
	}
	zlog.Logger = zlog.Logger.With().Caller().Logger()

	zerolog.DefaultContextLogger = &zlog.Logger
}

func GetSlogLogger() *slog.Logger {
	return slog.New(zerolog.NewSlogHandler(zlog.Logger))
}
