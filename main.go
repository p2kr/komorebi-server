package main

import (
	"context"
	"os"
	"os/signal"
	"syscall"
	"time"

	"komorebi-server/src/core"

	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
)

//go:generate swag fmt -d ./,./src/controllers
//go:generate swag init --v3.1 -d ./,./src/controllers
//go:generate go test -v ./tools -run "^TestGenerateTypes$"

//	@title			Komorebi Server API
//	@version		1.0
//	@description	Komorebi Server API documentation

//	@license.name	AGPL 3.0
//	@license.url	https://www.gnu.org/licenses/agpl-3.0.en.html

// @server	localhost:8080/api/v1
func main() {
	// init.
	core.Init()
	defer core.Defer()

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
