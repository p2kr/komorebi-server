package controllers

import (
	"errors"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"

	"komorebi-server/src/models"
	"komorebi-server/src/processors"

	"komorebi-server/configs"

	"github.com/labstack/echo/v5"
	zlog "github.com/rs/zerolog/log"
)

func StreamRoutes(g *echo.Group) {
	r := g.Group("/stream")

	r.Any("/video/:file", Stream)
}

func Stream(c *echo.Context) error {
	zlog.Debug().Str("file", c.Param("file")).Msg("streaming file")

	file, err := url.QueryUnescape(c.Param("file"))
	if err != nil || strings.TrimSpace(file) == "" {
		return fail(c, http.StatusBadRequest, errors.New("file parameter is required/invalid"))
	}

	_, err = os.Stat(file)
	if err != nil {
		return fail(c, http.StatusBadRequest, err)
	}

	// Verify if file is within vault
	fp, err := filepath.Abs(file)
	if err != nil {
		return fail(c, http.StatusUnauthorized, errors.New("invalid file path"))
	}

	vaultLoc, err := filepath.Abs(configs.GetConfig().Env.VaultLoc)
	if err != nil {
		return fail(c, http.StatusNotAcceptable, errors.New("invalid vault location"))
	}
	_, err = filepath.Rel(vaultLoc, fp)
	if err != nil || !strings.HasPrefix(fp, vaultLoc) {
		return fail(c, http.StatusUnauthorized, errors.New("file not in vault"))
	}

	// Stream the file.
	r, w := io.Pipe()
	defer r.Close()

	// I think process video is a blocking method.
	go func() {
		defer w.Close()
		err = processors.ProcessVideo(c.Request().Context(), models.VaultItem{FilePath: fp}, 0, w)
		if err != nil {
			zlog.Err(err).Str("file", file).Msg("processing failed")
		}
	}()

	return c.Stream(200, "video/mp4", r)
}
