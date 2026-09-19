package controllers

import (
	"context"
	"net/http"
	"strings"
	"sync"

	"komorebi-server/configs"
	"komorebi-server/src/db"
	"komorebi-server/src/models"

	"github.com/labstack/echo/v5"
	"github.com/rs/zerolog/log"
	"gorm.io/gorm"
	"resty.dev/v3"

	"github.com/maypok86/otter/v2"

	"github.com/vmihailenco/msgpack/v5"
)

type SuccessResponse struct {
	Success bool `json:"success"`
	Data    any  `json:"data"`
}

type FailureResponse struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
	Details []any  `json:"details,omitempty"`
}

func success(c *echo.Context, data any, customStatus ...int) error {
	if strings.Contains(c.Request().Header.Get("Accept"), "application/x-msgpack") {
		return successMsgPack(c, data)
	}
	status := http.StatusOK
	if len(customStatus) > 0 {
		status = customStatus[0]
	}
	return c.JSON(status, SuccessResponse{
		Success: true,
		Data:    data,
	})
}

func successMsgPack(c *echo.Context, data any) error {
	out, err := msgpack.Marshal(SuccessResponse{
		Success: true,
		Data:    data,
	})
	if err != nil {
		return err
	}
	return c.Blob(200, "application/x-msgpack", out)
}

func fail(c *echo.Context, status int, error error, details ...any) error {
	if strings.Contains(c.Request().Header.Get("Accept"), "application/x-msgpack") {
		return failMsgPack(c, status, error, details...)
	}
	return c.JSON(status, FailureResponse{
		Success: false,
		Message: error.Error(),
		Details: details,
	})
}

func failMsgPack(c *echo.Context, status int, error error, details ...any) error {
	out, err := msgpack.Marshal(FailureResponse{
		Success: false,
		Message: error.Error(),
		Details: details,
	})
	if err != nil {
		return err
	}
	return c.Blob(status, "application/x-msgpack", out)
}

var httpClient *resty.Client

func InitClient() {
	c := resty.NewWithTransportSettings(&resty.TransportSettings{MaxIdleConnsPerHost: 10})

	if configs.GetConfig().HttpClient.Debug {
		c.SetLogger(&RestyLogger{})
		c.SetDebug(true)
	}
	if configs.GetConfig().HttpClient.CurlCmd {
		c.SetCurlCmdGenerate(true)
	}

	c.SetHeaders(map[string]string{
		"User-Agent":      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
		"Accept":          "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
		"Accept-Language": "en-US,en;q=0.9",
	})

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

// Cache is global Cache for controllers
var Cache = sync.OnceValue(func() *otter.Cache[string, any] {
	c, err := otter.New[string, any](&otter.Options[string, any]{
		MaximumSize: 100,
	})
	if err != nil {
		log.Err(err).Msg("failed to initialize controller cache")
	}
	return c
})

func getCrawlerConfigs(ctx context.Context) ([]models.CrawlerConfig, bool) {
	r, ok := Cache().ComputeIfAbsent("configs", func() (any, bool) {
		var c []models.CrawlerConfig
		c, err := gorm.G[models.CrawlerConfig](db.GetDb()).Find(ctx)
		if err != nil {
			log.Err(err).Any("config from db", c).Msg("failed to get db config")
			return c, true
		} else {
			return c, false
		}
	})
	m, ok := r.([]models.CrawlerConfig)
	return m, ok
}
