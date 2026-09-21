package core

import (
	"context"

	"komorebi-server/configs"
	"komorebi-server/src/controllers"
	"komorebi-server/src/db"
	"komorebi-server/src/downloaders"
	"komorebi-server/src/workers"

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

	// Init resty client
	controllers.InitClient()

	// Init schedulers
	workers.InitScheduler()

	ctx := context.Background()
	// Restore jobs
	downloaders.RestoreJobs(ctx)

	log.Info().Msg("Initialized App")
}
