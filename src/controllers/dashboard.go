package controllers

import (
	"fmt"
	"net/http"

	"komorebi-server/src/adapters"

	"komorebi-server/src/db"
	"komorebi-server/src/models"

	"komorebi-server/src/dto"

	"github.com/labstack/echo/v5"
	"gorm.io/gorm"
)

func DashboardRoutes(g *echo.Group) {
	r := g.Group("/dashboard")

	r.POST("/:media_type", GetMedia)
}

func GetMedia(c *echo.Context) error {
	var params dto.MediaClientParams
	err := c.Bind(&params)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	// fetch user from db
	user, err := gorm.G[models.User](db.GetDb()).Where("id", params.UserId).First(c.Request().Context())
	if err != nil {
		return fail(c, http.StatusBadRequest, fmt.Errorf("unknown user: %w", err))
	}
	// Get specific adapter
	client := adapters.GetMediaClient(user.Provider, httpClient, &user)
	var list dto.PaginatedResponse

	path := c.Param("media_type")

	if path == "anime" {
		list, err = client.GetAnimeList(params)
	} else if path == "manga" {
		list, err = client.GetMangaList(params)
	} else {
		return fail(c, http.StatusBadRequest, fmt.Errorf("unknown media type: %s", path))
	}

	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}
	return success(c, list)
}
