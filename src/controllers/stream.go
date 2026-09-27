package controllers

import (
	"errors"
	"fmt"
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
	"gorm.io/gorm/clause"
)

func StreamRoutes(g *echo.Group) {
	r := g.Group("/stream")

	r.GET("/video/:vault_item_id/:file", Stream)
	r.GET("/subtitle/:subtitle_id", GetSubtitle)
}

func Stream(c *echo.Context) error {
	vaultItemId := c.Param("vault_item_id")
	requestedFile := c.Param("file")

	if strings.TrimSpace(requestedFile) == "" {
		return fail(c, http.StatusBadRequest, errors.New("no file specified"))
	}

	var item models.VaultItem
	err := db.GetDb().Preload(clause.Associations).Where("id = ?", vaultItemId).
		WithContext(c.Request().Context()).
		Find(&item).Error

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

	// 1. Dynamic Master Playlist
	if requestedFile == "master.m3u8" {
		c.Response().Header().Set("Content-Type", "application/vnd.apple.mpegurl")
		return c.String(http.StatusOK, processors.BuildMasterPlaylist(item))
	}

	// 2. Dynamic Segment Playlists
	if strings.HasPrefix(requestedFile, "stream_") && strings.HasSuffix(requestedFile, ".m3u8") {
		c.Response().Header().Set("Content-Type", "application/vnd.apple.mpegurl")
		return c.String(http.StatusOK, processors.BuildVODPlaylist(item, requestedFile))
	}

	// 3. Serve Segments and manage FFmpeg Lifecycle
	segmentNum := 0
	if before, ok := strings.CutSuffix(requestedFile, ".m4s"); ok {
		parts := strings.Split(before, "_")
		if len(parts) >= 3 {
			fmt.Sscanf(parts[len(parts)-1], "%d", &segmentNum)
		}
	}

	processors.TouchSession(item.Id)

	// Check if file exists. If it doesn't, ensure FFmpeg is running from this segment
	if _, err := os.Stat(filepath.Join(tempDir, requestedFile)); os.IsNotExist(err) {

		// HEURISTIC: Check if this is a "far seek" jump
		isSeek := false
		if segmentNum > 1 {
			// If we want segment 60, check if segment 59 exists.
			// If 59 doesn't exist, FFmpeg isn't currently generating this area.
			prevSegment := strings.Replace(
				requestedFile,
				fmt.Sprintf("_%d.m4s", segmentNum),
				fmt.Sprintf("_%d.m4s", segmentNum-1),
				1,
			)
			if _, err := os.Stat(filepath.Join(tempDir, prevSegment)); os.IsNotExist(err) {
				isSeek = true
			}
		}

		// If jumping ahead, kill the old session so GetOrCreateSession spins up a new one
		if isSeek {
			processors.RemoveSession(item.Id)
		}

		if ctx, isNew := processors.GetOrCreateSession(item.Id); isNew {
			go func() {
				defer processors.RemoveSession(item.Id)
				err = processors.ProcessVideo(ctx, item, tempDir, segmentNum)
				if err != nil {
					zlog.Err(err).Str("file", inputFile).Msg("processing failed")
				}
			}()
		}
	}

	_, err = backoff.Retry(c.Request().Context(), func() (any, error) {
		return os.Stat(filepath.Join(tempDir, requestedFile))
	}, backoff.WithMaxTries(10))
	if err != nil {
		return fail(c, http.StatusInternalServerError, err)
	}

	if strings.HasSuffix(requestedFile, ".m4s") || strings.HasSuffix(requestedFile, ".mp4") {
		// Cache media segments for 1 year and mark them as immutable
		c.Response().Header().Set("Cache-Control", "public, max-age=31536000, immutable")
	} else if strings.HasSuffix(requestedFile, ".m3u8") {
		// Never cache the playlist, especially while FFmpeg is still building it
		c.Response().Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
	}

	fsys := os.DirFS(tempDir)
	return c.FileFS(requestedFile, fsys)
}

func GetSubtitle(c *echo.Context) error {
	subId := c.Param("subtitle_id")

	if strings.TrimSpace(subId) == "" {
		return fail(c, http.StatusBadRequest, errors.New("no file specified"))
	}

	subtitle, err := gorm.G[models.VideoSubtitle](db.GetDb()).
		Where("id = ?", subId).First(c.Request().Context())
	if err != nil {
		return fail(c, http.StatusNotFound, errors.Join(err, errors.New("subtitle not found")))
	}

	item, ok := GetCachedVaultItems(subtitle.VaultItemId)
	if !ok {
		return fail(c, http.StatusNotFound, errors.New("vault item not found"))
	}

	tempDir := filepath.Join(os.TempDir(), KOMOREBI, item.Id.String())
	err = os.MkdirAll(tempDir, os.ModePerm)
	if err != nil {
		return fail(c, http.StatusInternalServerError, err)
	}

	ext := processors.AllowedSubtitles[strings.ToLower(subtitle.Format)]
	if ext == "" {
		return fail(c, http.StatusBadRequest, errors.New("unsupported subtitle format"))
	}
	requestedFile := subtitle.Id.String() + "." + ext
	requestedFilePath := filepath.Join(tempDir, requestedFile)

	if _, err := os.Stat(requestedFilePath); os.IsNotExist(err) {
		go func() {
			err = processors.ProcessSubtitle(c.Request().Context(), item, subtitle, tempDir)
			if err != nil {
				zlog.Err(err).Str("file", item.FilePath).Msg("processing failed")
			}
		}()
	}

	_, err = backoff.Retry(c.Request().Context(), func() (any, error) {
		return os.Stat(requestedFilePath)
	}, backoff.WithMaxTries(10))
	if err != nil {
		return fail(c, http.StatusNotFound, err)
	}

	//c.Response().Header().Set("Content-Type", "text/vtt; charset=utf-8")
	c.Response().Header().Set("Cache-Control", "public, max-age=31536000, immutable")

	fsys := os.DirFS(tempDir)
	return c.FileFS(requestedFile, fsys)
}
