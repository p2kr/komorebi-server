package controllers

import (
	"context"
	"komorebi-server/src/crawlers"
	"komorebi-server/src/db"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"net/http"

	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
	"gorm.io/gorm"
)

func CrawlerRoutes(g *echo.Group) {
	r := g.Group("/crawler")

	r.POST("/search", SearchQuery)
}

func SearchQuery(c *echo.Context) error {
	var params struct {
		MediaType dto.MediaType
		Query     string
	}

	err := c.Bind(&params)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	ctx := context.Background()
	configs, err := gorm.G[models.CrawlerConfig](db.GetDb()).Find(ctx)
	if err != nil || len(configs) == 0 {
		log.Err(err).Msg("No config found")
		return fail(c, http.StatusNotFound, err, "Configs", configs)
	}

	engine := crawlers.CrawlerEngine{
		Client:    httpClient,
		Query:     params.Query,
		MediaType: params.MediaType,
		Configs:   &configs,
	}

	res, err := engine.Crawl()
	if err != nil {
		log.Err(err).Msg("Failed to crawl")
		return fail(c, http.StatusInternalServerError, err)
	}
	return success(c, res)
}
