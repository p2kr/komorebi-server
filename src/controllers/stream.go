package controllers

import (
	"errors"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"komorebi-server/src/db"

	"komorebi-server/src/models"
	"komorebi-server/src/processors"

	"github.com/cenkalti/backoff/v7"
	"github.com/labstack/echo/v5"
	zlog "github.com/rs/zerolog/log"
	"gorm.io/gorm"
)

func StreamRoutes(g *echo.Group) {
	r := g.Group("/stream")

	r.Any("/video/:vault_item_id/*", Stream)
}

func Stream(c *echo.Context) error {
	vaultItemId := c.Param("vault_item_id")
	requestedFile := c.Param("*")

	if strings.TrimSpace(requestedFile) == "" {
		return fail(c, http.StatusBadRequest, errors.New("no file specified"))
	}

	item, err := gorm.G[models.VaultItem](db.GetDb()).
		Where("id = ?", vaultItemId).First(c.Request().Context())
	if err != nil || item.Id.String() != vaultItemId {
		return fail(c, http.StatusNotFound, errors.Join(err, errors.New("vault item not found")))
	}
	_, err = os.Stat(item.FilePath)
	if err != nil {
		return fail(c, http.StatusNotFound, errors.Join(err, errors.New("file not found")))
	}

	inputFile := item.FilePath

	tempDir := filepath.Join(os.TempDir(), KOMOREBI, item.Id.String())
	err = os.MkdirAll(tempDir, os.ModePerm)
	if err != nil {
		return fail(c, http.StatusInternalServerError, err)
	}
	//defer os.RemoveAll(tempDir) // TODO: Use cron job to remove files older than an hour.

	zlog.Debug().Any("vault_item_id", item.Id).
		Str("temp dir", tempDir).
		Str("input file", inputFile).Msg("streaming vault item")

	// Only start FFmpeg if they are requesting the main playlist, and it's not yet generating
	if strings.HasSuffix(requestedFile, ".m3u8") {
		if _, err := os.Stat(filepath.Join(tempDir, "index.m3u8")); os.IsNotExist(err) {
			go func() {
				err = processors.ProcessVideo(c.Request().Context(), item, tempDir)
				//err = processors.RemuxProcessor.Process(c.Request().Context(), item, tempDir)
				if err != nil {
					zlog.Err(err).Str("file", inputFile).Msg("processing failed")
				}
			}()
		}
	}

	_, err = backoff.Retry(c.Request().Context(), func() (any, error) {
		_, err := os.Stat(filepath.Join(tempDir, requestedFile))
		return nil, err
	}, backoff.WithMaxTries(10))
	if err != nil {
		return fail(c, http.StatusInternalServerError, err)
	}

	c.Response().Header().Set("Connection", "keep-alive")

	if strings.HasSuffix(requestedFile, ".m4s") || strings.HasSuffix(requestedFile, ".mp4") {
		// Cache media segments for 1 year, and mark them as immutable
		c.Response().Header().Set("Cache-Control", "public, max-age=31536000, immutable")
	} else if strings.HasSuffix(requestedFile, ".m3u8") {
		// Never cache the playlist, especially while FFmpeg is still building it
		c.Response().Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
	}

	fsys := os.DirFS(tempDir)
	return c.FileFS(requestedFile, fsys)
}
