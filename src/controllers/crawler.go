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

type ParsedTitleRequest struct {
	Title string `json:"title"`
}

// ParsedTitle godoc
//
//	@Summary		Parse a title
//	@Description	Parse a title and return the parsed components
//	@Tags			crawler
//	@Accept			json
//	@Produce		json
//	@Param			params	body		ParsedTitleRequest	true	"Title to parse"
//	@Success		200		{object}	SuccessResponse[any]
//	@Failure		400		{object}	FailureResponse
//	@Router			/crawler/parsed_title [post]
func ParsedTitle(c *echo.Context) error {
	var params ParsedTitleRequest
	err := c.Bind(&params)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}
	if params.Title == "" {
		return fail(c, http.StatusBadRequest, errors.New("title is required"))
	}

	parsedTitle := parsers.Parse(params.Title)
	if parsedTitle.Title.IsEmpty() || parsedTitle.Title.Equal(mapset.NewSet("")) {
		parsedTitle.Title = mapset.NewSet(params.Title)
	}
	return success(c, parsedTitle)
}

type SearchQueryRequest struct {
	MediaType dto.MediaType `json:"media_type"`
	Query     string        `json:"query"`
}

// SearchQuery godoc
//
//	@Summary		Search for media
//	@Description	Search crawlers for a specific query
//	@Tags			crawler
//	@Accept			json
//	@Produce		json
//	@Param			params	body		SearchQueryRequest	true	"Search parameters"
//	@Success		200		{object}	SuccessResponse[any]
//	@Failure		400		{object}	FailureResponse
//	@Failure		404		{object}	FailureResponse
//	@Failure		500		{object}	FailureResponse
//	@Router			/crawler/search [post]
func SearchQuery(c *echo.Context) error {
	var params SearchQueryRequest

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

	return success(c, res)
}
