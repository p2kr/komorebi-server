package core

import (
	"komorebi-server/configs"
	"komorebi-server/src/db"

	"github.com/rs/zerolog/log"
)

// Init Initializes all configuration, including reading from env.
func Init() {
	// Read config
	configs.LoadConfigs()

	// Setup Logger
	SetupLogger(configs.GetConfig())

	// Setup db
	db.SetupDb()

	log.Info().Msg("Initialized App")
}
