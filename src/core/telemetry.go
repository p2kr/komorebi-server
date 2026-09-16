package core

import (
	"komorebi-server/configs"
	"os"
	"strings"

	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

func SetupLogger(config configs.Config) {
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
		log.Logger = log.Output(zerolog.ConsoleWriter{Out: os.Stderr})
	}
}
