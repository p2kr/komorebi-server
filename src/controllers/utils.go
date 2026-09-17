package controllers

import (
	"komorebi-server/configs"
	"net/http"

	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
	"resty.dev/v3"
)

type SuccessResponse struct {
	Success bool `json:"success"`
	Data    any  `json:"data"`
}

type FailureResponse struct {
	Success bool   `json:"success"`
	Error   string `json:"error"`
	Details []any  `json:"details"`
}

func success(c *echo.Context, data any) error {
	return c.JSON(http.StatusOK, SuccessResponse{
		Success: true,
		Data:    data,
	})
}

func fail(c *echo.Context, status int, error error, details ...any) error {
	return c.JSON(status, FailureResponse{
		Success: false,
		Error:   error.Error(),
		Details: details,
	})
}

var httpClient *resty.Client

func InitClient() {
	c := resty.New()
	if configs.GetConfig().HttpClient.Debug {
		c.SetLogger(&RestyLogger{})
		c.SetDebug(true)
	}
	if configs.GetConfig().HttpClient.CurlCmd {
		c.SetCurlCmdGenerate(true)
	}

	httpClient = c
}

type RestyLogger struct{}

func (r *RestyLogger) Errorf(format string, v ...any) {
	log.Error().Msgf(format, v...)
}

func (r *RestyLogger) Warnf(format string, v ...any) {
	log.Warn().Msgf(format, v...)
}

func (r *RestyLogger) Debugf(format string, v ...any) {
	log.Debug().Msgf(format, v...)
}
