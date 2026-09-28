package controllers

import (
	"context"
	"errors"
	"fmt"
	"net/http"

	"komorebi-server/src/adapters"
	"komorebi-server/src/db"
	"komorebi-server/src/dto"
	"komorebi-server/src/models"

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
	r.POST("/oauth/exchange", ExchangeOauthToken)
}

// AddUser godoc
//
//	@Summary		Add a new user
//	@Description	Add a new user and validate them
//	@Tags			user
//	@Accept			json
//	@Produce		json
//	@Param			user	body		models.User	true	"User Data"
//	@Success		200		{object}	SuccessResponse[models.User]
//	@Failure		400		{object}	FailureResponse
//	@Failure		406		{object}	FailureResponse
//	@Failure		500		{object}	FailureResponse
//	@Router			/user/add [post]
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

// GetUsers godoc
//
//	@Summary		Get all users
//	@Description	Get a list of all users
//	@Tags			user
//	@Produce		json
//	@Success		200	{object}	SuccessResponse[[]models.User]
//	@Failure		404	{object}	FailureResponse
//	@Router			/user/all [get]
//	@Router			/user/all [post]
func GetUsers(c *echo.Context) error {
	ctx := context.Background()
	users, err := gorm.G[models.User](db.GetDb()).Find(ctx)
	if err != nil {
		return fail(c, http.StatusNotFound, err)
	}
	return success(c, users)
}

// DeleteUser godoc
//
//	@Summary		Delete a user
//	@Description	Delete a user by their ID
//	@Tags			user
//	@Accept			json
//	@Produce		json
//	@Param			request	body		map[string]string	true	"User ID"
//	@Success		200		{object}	SuccessResponse[string]
//	@Failure		400		{object}	FailureResponse
//	@Failure		404		{object}	FailureResponse
//	@Failure		500		{object}	FailureResponse
//	@Router			/user/delete [post]
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
	client := adapters.GetMediaClient(user.Provider, httpClient, user)

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

// ExchangeOauthToken godoc
//
//	@Summary		Exchange OAuth token
//	@Description	Exchange OAuth token for a user
//	@Tags			user
//	@Accept			json
//	@Produce		json
//	@Param			params	body		dto.ExchangeOauthParams	true	"OAuth Exchange Params"
//	@Success		200		{object}	SuccessResponse[any]
//	@Failure		400		{object}	FailureResponse
//	@Router			/user/oauth/exchange [post]
func ExchangeOauthToken(c *echo.Context) error {
	var params dto.ExchangeOauthParams
	err := c.Bind(&params)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}
	errs := dto.IExchangeOauthParams.Validate(&params)
	if errs != nil {
		return fail(c, http.StatusBadRequest, fmt.Errorf("%v", zog.Issues.Flatten(errs)))
	}

	client := adapters.GetMediaClient(params.Provider, httpClient, nil)
	token, err := client.ExchangeOauthToken(params.Code, params.CodeVerifier)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}
	return success(c, token)
}
