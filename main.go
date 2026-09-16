package main

import (
	"komorebi-server/src/core"

	"github.com/rs/zerolog/log"
)

func main() {
	// init.
	core.Init()

	// setup web server
	e := core.GetServer()

	if err := e.Start(":8080"); err != nil {
		log.Err(err).Msg("failed to start server")
	}
}
