package core

import (
	"komorebi-server/configs"
	"komorebi-server/src/controllers"
	"path/filepath"

	"github.com/labstack/echo/v5"
	"github.com/labstack/echo/v5/middleware"
	"github.com/rs/zerolog/log"
)

func GetServer() *echo.Echo {
	e := echo.New()

	e.Logger = GetSlogLogger()

	e.Use(middleware.RequestLogger())

	staticPath, err := filepath.Abs(configs.GetConfig().Frontend.StaticPath)
	if err != nil {
		log.Err(err).Msg("Failed to get static path")
	} else {
		log.Info().Str("staticPath", staticPath).Msg("Serving static files from [staticPath]")
	}

	e.Use(middleware.Recover())
	e.Use(middleware.CSRFWithConfig(middleware.CSRFConfig{
		Skipper: func(_ *echo.Context) bool {
			return configs.GetConfig().Env.AppEnv == "dev"
		},
	}))
	e.Use(middleware.Static(staticPath))

	// Static files
	e.Static("/", staticPath)

	// Map Routers
	g := e.Group("/api/v1")

	controllers.UserRoutes(g) // Add routes below

	return e
}
