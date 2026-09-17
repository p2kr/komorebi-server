package controllers

import (
	"context"
	"errors"
	"fmt"
	"komorebi-server/src/adapters"
	"komorebi-server/src/db"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"
	"net/http"

	"github.com/Oudwins/zog"
	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
	"gorm.io/gorm"
)

func UserRoutes(g *echo.Group) {
	r := g.Group("/user")

	r.POST("/add", AddUser)
	r.Match([]string{"GET", "POST"}, "/all", GetUsers)
	r.POST("/delete", DeleteUser)
}

func AddUser(c *echo.Context) error {
	var user models.User
	if err := c.Bind(&user); err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	errs := models.IUser.Validate(&user)
	if errs != nil {
		return fail(c, http.StatusNotAcceptable, fmt.Errorf("%v", zog.Issues.Flatten(errs)))
	}

	err := validateUser(&user)
	if err != nil {
		return fail(c, http.StatusNotAcceptable, err)
	}

	// Save to db
	err = user.InsertOrUpdate(db.GetDb())
	if err != nil {
		return fail(c, http.StatusInternalServerError, err)
	}

	return success(c, user)
}

func GetUsers(c *echo.Context) error {
	ctx := context.Background()
	users, err := gorm.G[models.User](db.GetDb()).Find(ctx)
	if err != nil {
		return fail(c, http.StatusNotFound, err)
	}
	return success(c, users)
}

func DeleteUser(c *echo.Context) error {
	ctx := context.Background()

	var user struct {
		Id string `json:"user_id"`
	}
	err := c.Bind(&user)
	if err != nil {
		return fail(c, http.StatusBadRequest, err, "user", user)
	}

	log.Debug().Any("user id", user.Id).Msg("Deleting user")

	rowCount, err := gorm.G[models.User](db.GetDb()).Where("id = ?", user.Id).Delete(ctx)
	if err != nil {
		return fail(c, http.StatusInternalServerError, err)
	}
	if rowCount == 0 {
		return fail(c, http.StatusNotFound, errors.New("no user deleted"))
	}
	return success(c, user.Id)
}

func validateUser(user *models.User) error {
	client := adapters.GetMediaClient(dto.MediaProvider(user.Provider), httpClient, user)

	if user.AccessToken != nil {
		log.Info().Msg("Fetching username and avatar url for user")
		err := client.ValidateNewUser(*user.AccessToken)
		if err != nil {
			return err
		}
		return nil
	}

	limit := 1
	params := dto.MediaClientParams{
		Limit: &limit,
	}
	_, err := client.GetAnimeList(params)
	if err == nil {
		log.Info().Msg("Validated user by anime list")
		return nil
	}

	_, err = client.GetMangaList(params)
	if err == nil {
		log.Info().Msg("Validated user by manga list")
		return nil
	}

	return err
}
