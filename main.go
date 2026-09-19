package main

import (
	"context"
	"os"
	"os/signal"
	"syscall"
	"time"

	"komorebi-server/src/core"
	"komorebi-server/src/workers"

	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
)

func main() {
	// init.
	core.Init()

	// setup scheduler
	workers.InitScheduler()
	defer workers.CloseScheduler()

	// setup web server
	e := core.GetServer()
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	st := echo.StartConfig{
		Address:         ":8080",
		GracefulTimeout: 5 * time.Second,
	}

	if err := st.Start(ctx, e); err != nil {
		log.Err(err).Msg("failed to start server")
	}
}
