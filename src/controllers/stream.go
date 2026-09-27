package controllers

import (
	"errors"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"uuid"

	"komorebi-server/src/models"

	"github.com/labstack/echo/v5"
	zlog "github.com/rs/zerolog/log"
)

func StreamRoutes(g *echo.Group) {
	r := g.Group("/stream")

	r.GET("/video/:vault_item_id/:file", Stream)
}

func Stream(c *echo.Context) error {
	vaultItemId := c.Param("vault_item_id")
	requestedFile := c.Param("file")

	itemId, err := uuid.Parse(vaultItemId)
	if err != nil {
		return fail(c, http.StatusBadRequest, errors.New("unsupported vault id"))
	}
	if strings.TrimSpace(requestedFile) == "" {
		return fail(c, http.StatusBadRequest, errors.New("no file specified"))
	}

	item, ok := GetCachedVaultItem(itemId)

	if !ok || item.Id.String() != vaultItemId {
		return fail(c, http.StatusNotFound, errors.Join(err, errors.New("vault item not found")))
	}

	if item.Status != models.DownloadStatusReady {
		return fail(c, http.StatusServiceUnavailable, errors.New("vault item is processing"))
	}

	targetDir := filepath.Join(filepath.Dir(item.FilePath), item.Id.String())
	targetPath := filepath.Join(targetDir, requestedFile)

	if _, err := os.Stat(targetPath); os.IsNotExist(err) {
		return fail(c, http.StatusNotFound, errors.New("file not found"))
	}

	// Set cache control for all successfully extracted static media files
	c.Response().Header().Set("Cache-Control", "public, max-age=31536000, immutable")

	zlog.Debug().Str("targetPath", targetPath).Str("targetDir", targetDir).Msg("streaming file")
	fsys := os.DirFS(targetDir)
	return c.FileFS(requestedFile, fsys)
}
