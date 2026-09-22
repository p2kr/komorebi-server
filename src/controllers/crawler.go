package controllers

import (
	"context"
	"errors"
	"fmt"
	"net/http"

	"komorebi-server/src/crawlers"
	"komorebi-server/src/dto"
	"komorebi-server/src/parsers"

	mapset "github.com/deckarep/golang-set/v3"
	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
)

func CrawlerRoutes(g *echo.Group) {
	// Load crawler configs in background
	ctx := context.Background()
	go getCrawlerConfigs(ctx)

	r := g.Group("/crawler")

	r.POST("/search", SearchQuery)
	r.POST("/parsed_title", ParsedTitle)
}

func ParsedTitle(c *echo.Context) error {
	var params struct {
		Title string `json:"title,omitempty"`
	}
	err := c.Bind(&params)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}
	if params.Title == "" {
		return fail(c, http.StatusBadRequest, errors.New("title is required"))
	}

	parser := parsers.TitleParser{Ctx: c.Request().Context()}

	parsedTitle := parser.Parse(params.Title)
	if parsedTitle == nil {
		parsedTitle = &dto.ParsedTitle{Title: mapset.NewSet(params.Title)}
	} else if parsedTitle.Title.IsEmpty() || parsedTitle.Title.Equal(mapset.NewSet("")) {
		parsedTitle.Title = mapset.NewSet(params.Title)
	}
	return success(c, parsedTitle)
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
	cfg, ok := getCrawlerConfigs(ctx)
	if !ok || len(cfg) == 0 {
		log.Err(err).Any("configs", cfg).Msg("No config found")
		return fail(c, http.StatusNotFound,
			fmt.Errorf("%s, empty crawler configs", err), "Configs", cfg)
	}

	engine := crawlers.CrawlerEngine{
		Client:    httpClient,
		Query:     params.Query,
		MediaType: params.MediaType,
		Configs:   cfg,
	}

	res, err := engine.Crawl()
	if err != nil {
		log.Err(err).Msg("Failed to crawl")
		return fail(c, http.StatusInternalServerError, err)
	}
	tp := parsers.TitleParser{Ctx: ctx}
	tp.ParseMany(res)

	return success(c, res)
}
