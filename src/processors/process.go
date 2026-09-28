package processors

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	ffmpeg "github.com/u2takey/ffmpeg-go"
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

const hlsAudioGroup = "audio"

type PostProcess struct{}

var PostProcessor = &PostProcess{}

// moveFile renames src→dst, falling back to copy+delete on cross-device errors.
func moveFile(src, dst string) error {
	if err := os.Rename(src, dst); err == nil {
		return nil
	}
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()
	out, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer out.Close()
	if _, err = io.Copy(out, in); err != nil {
		return err
	}
	in.Close()
	return os.Remove(src)
}

// moveDir moves src directory to dst. Attempts an atomic rename first; falls back
// to file-by-file copy on cross-device failure. Any pre-existing dst is removed first.
func moveDir(src, dst string) error {
	_ = os.RemoveAll(dst)
	if err := os.Rename(src, dst); err == nil {
		return nil
	}
	if err := os.MkdirAll(dst, os.ModePerm); err != nil {
		return err
	}
	entries, err := os.ReadDir(src)
	if err != nil {
		return err
	}
	for _, e := range entries {
		if err = moveFile(filepath.Join(src, e.Name()), filepath.Join(dst, e.Name())); err != nil {
			return err
		}
	}
	return os.RemoveAll(src)
}

func audioDisplayName(title, lang, fallback string) string {
	var name string
	switch {
	case title != "":
		name = title
	case lang != "":
		name = lang
	default:
		name = fallback
	}
	return strings.ReplaceAll(name, `"`, ``)
}

var nameAttrRegex *regexp.Regexp

func init() {
	var err error
	nameAttrRegex, err = regexp.Compile(`NAME="[^"]*"`)
	if err != nil {
		zlog.Fatal().Err(err).Msg("failed to compile NAME attribute regex")
	}
}

// rewriteMasterNames swaps the FFmpeg-generated NAME="..." values in master.m3u8 for their
// display equivalents (which may contain spaces). It maps them using the URI attribute.
func rewriteMasterNames(masterPath string, renames map[string]string) error {
	if len(renames) == 0 {
		return nil
	}
	data, err := os.ReadFile(masterPath)
	if err != nil {
		return fmt.Errorf("reading master playlist: %w", err)
	}

	lines := strings.Split(string(data), "\n")

	for i, line := range lines {
		if strings.HasPrefix(line, "#EXT-X-MEDIA:TYPE=AUDIO") {
			for safe, displayName := range renames {
				uriStr := fmt.Sprintf(`URI="%s.m3u8"`, safe)
				if strings.Contains(line, uriStr) {
					lines[i] = nameAttrRegex.ReplaceAllString(line, fmt.Sprintf(`NAME="%s"`, displayName))
					break
				}
			}
		}
	}
	return os.WriteFile(masterPath, []byte(strings.Join(lines, "\n")), 0o644)
}

// runStream executes an ffmpeg stream with context propagation, stderr capture,
// and an optional working directory for relative output paths.
// s.Context must be set BEFORE calling OverWriteOutput/WithErrorOutput since both
// derive child contexts from s.Context in place.
func runStream(ctx context.Context, s *ffmpeg.Stream, input, label, dir string) error {
	var stderr bytes.Buffer
	// GlobalArgs returns a new *Stream with context.Background(); set Context after
	// so OverWriteOutput/WithErrorOutput chain derives from ctx, not background.
	s = s.GlobalArgs("-v", "warning", "-nostdin")
	s.Context = ctx
	cmp := s.OverWriteOutput().WithErrorOutput(&stderr).Compile()
	if dir != "" {
		cmp.Dir = dir
	}
	zlog.Debug().Str("label", label).Str("dir", dir).Msg("starting ffmpeg")
	if err := cmp.Run(); err != nil {
		zlog.Err(err).Str("file", input).Str("stderr", stderr.String()).Msgf("ffmpeg %s failed", label)
		return fmt.Errorf("ffmpeg %s failed: %w", label, err)
	}
	return nil
}

func (p *PostProcess) ProcessOne(ctx context.Context, item *models.VaultItem) (err error) {
	if item.Status == models.DownloadStatusReady {
		return nil
	}

	input := item.FilePath
	outputDir := filepath.Join(filepath.Dir(input), item.Id.String())

	tempDir, err := os.MkdirTemp("", item.Id.String())
	if err != nil {
		return fmt.Errorf("creating temp dir: %w", err)
	}
	defer func() {
		if err != nil {
			os.RemoveAll(tempDir)
			os.RemoveAll(outputDir) // clean up any partial moveDir output
		}
	}()

	// Codec selection: all-or-nothing — one incompatible track forces transcoding of all tracks of that type.
	videoCodec := "copy"
	for _, v := range item.VideoTracks {
		if AllowedVideoCodecs[strings.ToLower(v.Codec)] == "" {
			videoCodec = "libx264"
			break
		}
	}
	audioCodec := "copy"
	for _, a := range item.AudioTracks {
		if AllowedAudioCodecs[strings.ToLower(a.Codec)] == "" {
			audioCodec = "aac"
			break
		}
	}

	zlog.Debug().
		Str("input", input).
		Str("videoCodec", videoCodec).
		Str("audioCodec", audioCodec).
		Int("videoTracks", len(item.VideoTracks)).
		Int("audioTracks", len(item.AudioTracks)).
		Int("subtitles", len(item.VideoSubtitles)).
		Msg("beginning extraction")

	// Build multi-variant HLS A/V output.
	inStream := ffmpeg.Input(input)
	avStreams := make([]*ffmpeg.Stream, 0, len(item.VideoTracks)+len(item.AudioTracks))
	varStreamParts := make([]string, 0, len(item.VideoTracks)+len(item.AudioTracks))
	audioRenames := make(map[string]string) // safe→display for master.m3u8 post-processing

	agroup := ""
	if len(item.AudioTracks) > 0 {
		agroup = ",agroup:" + hlsAudioGroup
	}

	for i, v := range item.VideoTracks {
		name := fmt.Sprintf("video_%d", i)
		avStreams = append(avStreams, inStream.Get(fmt.Sprintf("%d", v.StreamIdx)))
		varStreamParts = append(varStreamParts,
			fmt.Sprintf("v:%d%s,name:%s", i, agroup, name),
		)
		item.VideoTracks[i].FilePath = filepath.Join(outputDir, name+".m3u8")
	}

	for i, a := range item.AudioTracks {
		name := fmt.Sprintf("audio_%d", i) // index-based: unique even with duplicate lang/title
		display := audioDisplayName(a.Title, a.Lang, name)
		avStreams = append(avStreams, inStream.Get(fmt.Sprintf("%d", a.StreamIdx)))
		if display != name {
			audioRenames[name] = display
		}
		part := fmt.Sprintf("a:%d%s,name:%s", i, agroup, name)
		if a.Lang != "" {
			part += ",language:" + a.Lang
		}
		if a.IsDefault {
			part += ",default:yes"
		} else {
			part += ",default:no"
		}
		varStreamParts = append(varStreamParts, part)
		item.AudioTracks[i].FilePath = filepath.Join(outputDir, name+".m3u8")
	}

	// Subtitle extraction runs as a separate FFmpeg process (ASS/SSA cannot be muxed into fmp4/HLS).
	// Output paths are relative; resolved via cmp.Dir = tempDir.
	subInStream := ffmpeg.Input(input)
	subOutputStreams := make([]*ffmpeg.Stream, 0, len(item.VideoSubtitles))

	for i, sub := range item.VideoSubtitles {
		ext := AllowedSubtitles[strings.ToLower(sub.Format)]
		if ext == "" {
			continue
		}
		subName := fmt.Sprintf("sub_%d.%s", i, ext)
		subOutputStreams = append(subOutputStreams,
			subInStream.Get(fmt.Sprintf("%d", sub.StreamIdx)).Output(subName, ffmpeg.KwArgs{
				"c:s": "copy",
			}),
		)
		item.VideoSubtitles[i].FilePath = filepath.Join(outputDir, subName)
	}

	if len(avStreams) == 0 && len(subOutputStreams) == 0 {
		return errors.New("no valid streams found to extract")
	}

	eg, egCtx := errgroup.WithContext(ctx)
	eg.SetLimit(2)

	if len(avStreams) > 0 {
		avKwargs := ffmpeg.KwArgs{
			"f":                    "hls",
			"hls_segment_type":     "fmp4",
			"hls_flags":            "single_file",
			"hls_time":             10,
			"hls_playlist_type":    "vod",
			"master_pl_name":       "master.m3u8",
			"hls_segment_filename": "%v.mp4",
			"var_stream_map":       strings.Join(varStreamParts, " "),
			"threads":              "0",
			"c:v":                  videoCodec,
			"c:a":                  audioCodec,
		}
		if videoCodec != "copy" {
			avKwargs["preset"] = "faster"
			avKwargs["crf"] = "23"
			avKwargs["g"] = "250"
			avKwargs["keyint_min"] = "25"
		}

		avOutput := ffmpeg.Output(avStreams, "%v.m3u8", avKwargs)
		eg.Go(func() error {
			return runStream(egCtx, avOutput, input, "av", tempDir)
		})
	}

	if len(subOutputStreams) > 0 {
		subOutput := ffmpeg.MergeOutputs(subOutputStreams...)
		eg.Go(func() error {
			return runStream(egCtx, subOutput, input, "subtitles", tempDir)
		})
	}

	if err = eg.Wait(); err != nil {
		return err
	}

	if len(avStreams) > 0 {
		if err = rewriteMasterNames(filepath.Join(tempDir, "master.m3u8"), audioRenames); err != nil {
			return err
		}
	}

	zlog.Debug().Str("from", tempDir).Str("to", outputDir).Msg("moving output to final destination")
	if err = moveDir(tempDir, outputDir); err != nil {
		return fmt.Errorf("moving output: %w", err)
	}

	// Delete original only after everything succeeds.
	_, _ = backoff.Retry(ctx, func() (any, error) {
		e := os.Remove(input)
		if e != nil && !errors.Is(e, os.ErrNotExist) {
			return nil, e
		}
		return nil, nil
	}, backoff.WithMaxTries(5))

	item.Status = models.DownloadStatusReady
	return nil
}
