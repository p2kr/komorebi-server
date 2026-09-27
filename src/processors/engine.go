package processors

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"komorebi-server/src/models"

	zlog "github.com/rs/zerolog/log"
	ffmpeg "github.com/u2takey/ffmpeg-go"
)

type Processor interface {
	Process(ctx context.Context, item models.VaultItem, outputDir string) error
}

var AllowedSubtitles = map[string]string{
	"subrip": "srt",
	"srt":    "srt",
	"vtt":    "vtt",
	"webvtt": "vtt",
	"ass":    "ass",
	"ssa":    "ass",
}

var RemuxProcessor = remux{}

func ProcessVideo(ctx context.Context, item models.VaultItem, outputDir string) error {
	if item.FilePath == "" {
		return errors.New("invalid file path")
	}
	err := RemuxProcessor.Process(ctx, item, outputDir)
	if err != nil {
		return err
	}
	// Rename audio tracks names in m3u8 file
	masterFile := filepath.Join(outputDir, "master.m3u8")
	contents, err := os.ReadFile(masterFile)
	if err == nil {
		masterText := string(contents)
		for i, audioTrack := range item.AudioTracks {
			searchStr := fmt.Sprintf(`NAME="audio_%d"`, i+1)
			title := audioTrack.Title
			if title == "" {
				title = audioTrack.Lang
				if title == "" {
					continue
				}
			}
			replaceStr := fmt.Sprintf(`NAME="%s"`, title)
			masterText = strings.Replace(masterText, searchStr, replaceStr, 1)
		}
		os.WriteFile(masterFile, []byte(masterText), 0o644)
	}

	return nil
}

func ProcessSubtitle(ctx context.Context,
	item models.VaultItem, sub models.VideoSubtitle, outputDir string,
) error {
	ext := AllowedSubtitles[strings.ToLower(sub.Format)]
	if ext == "" {
		return errors.New("unsupported subtitle format")
	}

	input := item.FilePath
	outputFile := fmt.Sprintf("%s.%s", sub.Id.String(), ext)
	tempOutputFile := "temp_" + outputFile

	stream := ffmpeg.Input(input).
		Output(tempOutputFile, ffmpeg.KwArgs{
			"map": fmt.Sprintf("0:%d", sub.StreamIdx),
			"c:s": "copy",
		})
	stream.Context = ctx

	var stderr bytes.Buffer
	cmd := stream.OverWriteOutput().
		WithErrorOutput(&stderr).
		Compile()

	// Run inside the temp directory
	cmd.Dir = outputDir

	err := cmd.Run()

	zlog.Err(err).Str("file", input).
		Str("output", stderr.String()).
		Msg("ffmpeg subtitle extraction done")

	if err != nil {
		return err
	}

	// Rename it atomically when done
	err = os.Rename(filepath.Join(outputDir, tempOutputFile), filepath.Join(outputDir, outputFile))
	if err != nil {
		zlog.Err(err).Str("file", input).Msg("rename failed")
	}
	return nil
}
