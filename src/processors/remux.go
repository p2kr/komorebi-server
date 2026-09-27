package processors

import (
	"bytes"
	"context"
	"fmt"
	"strings"

	"komorebi-server/src/models"

	zlog "github.com/rs/zerolog/log"
	ffmpeg "github.com/u2takey/ffmpeg-go"
)

type remux struct{}

func (r *remux) Process(ctx context.Context, item models.VaultItem, outputDir string) error {
	input := item.FilePath
	maps := []string{"0:v:0"}

	audioMapped := 0
	var audioMaps []string

	for i, a := range item.AudioTracks {
		lang := a.Lang
		if lang == "" {
			lang = strings.ReplaceAll(a.Title, " ", "_")
			if lang == "" {
				lang = fmt.Sprintf("Audio_%d", audioMapped+1)
			}
		}
		maps = append(maps, fmt.Sprintf("0:a:%d", i))
		audioMaps = append(audioMaps,
			fmt.Sprintf("a:%d,agroup:a,language:%s", audioMapped, lang))
		audioMapped++
	}

	var varStreamMap strings.Builder
	varStreamMap.WriteString("v:0")
	if audioMapped > 0 {
		varStreamMap.WriteString(",agroup:a")
	}
	for _, am := range audioMaps {
		varStreamMap.WriteString(" " + am)
	}

	kwargs := ffmpeg.KwArgs{
		"f":                 "hls",
		"hls_time":          "10",
		"hls_playlist_type": "vod",
		"hls_segment_type":  "fmp4",
		"master_pl_name":    "master.m3u8",
		"var_stream_map":    varStreamMap.String(),
		"hls_flags":         "temp_file",
		"map":               maps,
		"c:v":               "copy",
		"c:a":               "copy",
	}

	output := "stream_%v.m3u8"
	stream := ffmpeg.Input(input, ffmpeg.KwArgs{}).Output(output, kwargs)
	stream.Context = ctx

	var stderr bytes.Buffer
	cmd := stream.OverWriteOutput().
		WithErrorOutput(&stderr).
		// GlobalArgs("-v error").
		Compile()

	cmd.Dir = outputDir
	zlog.Debug().Str("file", input).Strs("command", cmd.Args).
		Msg("ffmpeg generating master playlist and segments")

	err := cmd.Run()
	zlog.Err(err).Str("file", input).Str("output", stderr.String()).Msg("ffmpeg done")
	if err != nil {
		return err
	}

	return nil
}
