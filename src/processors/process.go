package processors

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"golang.org/x/sync/errgroup"

	"github.com/cenkalti/backoff/v7"

	"komorebi-server/src/models"

	zlog "github.com/rs/zerolog/log"
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

var AllowedVideoCodecs = map[string]string{
	"h264": "mp4",
	"hevc": "mp4",
	"h265": "mp4",
	"vp9":  "mp4",
	"av1":  "mp4",
}

var AllowedAudioCodecs = map[string]string{
	"aac":  "m4a",
	"mp3":  "mp3",
	"opus": "webm",
}

type PostProcess struct{}

var PostProcessor = &PostProcess{}

type fileMapping struct {
	Temp string
	Dest string
}

func moveFile(sourcePath, destPath string) error {
	if err := os.Rename(sourcePath, destPath); err == nil {
		return nil
	}

	// Fallback to copy+delete for cross-device rename errors
	src, err := os.Open(sourcePath)
	if err != nil {
		return err
	}
	defer src.Close()

	dst, err := os.Create(destPath)
	if err != nil {
		return err
	}
	defer dst.Close()

	if _, err := io.Copy(dst, src); err != nil {
		return err
	}

	src.Close()
	return os.Remove(sourcePath)
}

func (p *PostProcess) ProcessOne(ctx context.Context, item *models.VaultItem) (err error) {
	if item.Status == models.DownloadStatusReady {
		return nil
	}

	input := item.FilePath
	outputDir := filepath.Join(filepath.Dir(input), item.Id.String())
	os.MkdirAll(outputDir, os.ModePerm)

	// Pre-allocate to reduce GC pressure
	estimatedOutputs := len(item.VideoTracks) + len(item.AudioTracks) + len(item.VideoSubtitles)
	args := make([]string, 0, 3+estimatedOutputs*6)
	args = append(args, "-y", "-i", input)
	mappings := make([]fileMapping, 0, estimatedOutputs)

	// Guarantee cleanup of any partial or temporary files if the process fails
	defer func() {
		if err != nil {
			for _, m := range mappings {
				os.Remove(m.Temp)
				os.Remove(m.Dest)
			}
		}
	}()

	// 1. Video
	for i, v := range item.VideoTracks {
		videoCodec := "copy"
		vc := strings.ToLower(v.Codec)
		videoExt := AllowedVideoCodecs[vc]

		if videoExt == "" {
			videoCodec = "libx264"
			videoExt = "mp4"
		}

		videoName := fmt.Sprintf("video_%d.%s", i, videoExt)

		tempPath := filepath.Join(os.TempDir(), fmt.Sprintf("komorebi_%s_%s", item.Id.String(), videoName))
		destPath := filepath.Join(outputDir, videoName)

		args = append(args, "-map", fmt.Sprintf("0:%d", v.StreamIdx), "-c:v", videoCodec, tempPath)
		mappings = append(mappings, fileMapping{Temp: tempPath, Dest: destPath})

		item.VideoTracks[i].FilePath = destPath
	}

	// 2. Audio
	for i, a := range item.AudioTracks {
		audioCodec := "copy"
		ac := strings.ToLower(a.Codec)
		audioExt := AllowedAudioCodecs[ac]

		if audioExt == "" {
			audioCodec = "aac"
			audioExt = "m4a"
		}

		audioName := fmt.Sprintf("audio_%d.%s", i, audioExt)
		tempPath := filepath.Join(os.TempDir(), fmt.Sprintf("komorebi_%s_%s", item.Id.String(), audioName))
		destPath := filepath.Join(outputDir, audioName)

		args = append(args, "-map", fmt.Sprintf("0:%d", a.StreamIdx), "-c:a", audioCodec, tempPath)
		mappings = append(mappings, fileMapping{Temp: tempPath, Dest: destPath})

		item.AudioTracks[i].FilePath = destPath
	}

	// 3. Subtitles
	for i, sub := range item.VideoSubtitles {
		ext := AllowedSubtitles[strings.ToLower(sub.Format)]
		if ext == "" {
			continue
		}

		subName := fmt.Sprintf("sub_%d.%s", i, ext)
		tempPath := filepath.Join(os.TempDir(), fmt.Sprintf("komorebi_%s_%s", item.Id.String(), subName))
		destPath := filepath.Join(outputDir, subName)

		args = append(args, "-map", fmt.Sprintf("0:%d", sub.StreamIdx), "-c:s", "copy", tempPath)
		mappings = append(mappings, fileMapping{Temp: tempPath, Dest: destPath})

		item.VideoSubtitles[i].FilePath = destPath
	}

	if len(mappings) == 0 {
		return errors.New("no valid streams found to extract")
	}

	// Run single FFmpeg process
	cmd := exec.CommandContext(ctx, "ffmpeg", args...)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr

	err = cmd.Run()
	if err != nil {
		zlog.Err(err).Str("file", input).Str("output", stderr.String()).Msg("ffmpeg multi-extraction failed")
		err = errors.Join(errors.New("ffmpeg multi-extraction failed"), err)
		return err
	}

	// Rename temp files to final destination concurrently
	eg, _ := errgroup.WithContext(ctx)
	for _, m := range mappings {
		eg.Go(func() error {
			os.Remove(m.Dest) // Ensure destination is clear to prevent 'file exists' errors on Windows
			if moveErr := moveFile(m.Temp, m.Dest); moveErr != nil {
				zlog.Err(moveErr).Str("temp", m.Temp).Str("dest", m.Dest).Msg("failed to move extracted file")
				return fmt.Errorf("failed to move extracted files: %w", moveErr)
			}
			return nil
		})
	}
	if err = eg.Wait(); err != nil {
		return err
	}

	// Delete original ONLY if all processing steps succeed
	_, _ = backoff.Retry(context.Background(), func() (any, error) {
		err := os.Remove(input)
		if err != nil && !errors.Is(err, os.ErrNotExist) {
			return nil, err
		}
		return nil, nil
	}, backoff.WithMaxTries(5))

	item.Status = models.DownloadStatusReady
	return nil
}
