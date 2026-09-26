package processors

import (
	"bytes"
	"context"

	"komorebi-server/src/models"

	zlog "github.com/rs/zerolog/log"
	ffmpeg "github.com/u2takey/ffmpeg-go"
)

type remux struct{}

func (r *remux) Process(ctx context.Context,
	item models.VaultItem, outputDir string,
) error {
	input := item.FilePath
	output := "index.m3u8"

	stream := ffmpeg.Input(input, ffmpeg.KwArgs{}).Output(output, ffmpeg.KwArgs{
		"f":                 "hls",
		"hls_time":          "10",
		"hls_playlist_type": "vod",
		"hls_segment_type":  "fmp4",
		//"movflags": "frag_keyframe+empty_moov+default_base_moof",
		//"movflags": "frag_keyframe+delay_moov+default_base_moof",
		//"map": []string{"0:v?", "0:a?"},
		//"map":      []string{"0:v?", "0:a?", "0:s:0?"},
		"c:v": "copy",
		"c:a": "copy",
		//"c:s":      "mov_text",
		//"c:s": "webvtt",
	})
	stream.Context = ctx

	var stderr bytes.Buffer
	cmp := stream.OverWriteOutput().
		WithErrorOutput(&stderr).
		GlobalArgs("-progress", "-v error").
		Compile()

	cmp.Dir = outputDir

	zlog.Debug().Str("file", input).Strs("command", stream.GetArgs()).Msg("ffmpeg running")

	err := cmp.Run()

	zlog.Err(err).Str("file", input).Str("output", stderr.String()).Msg("ffmpeg done")
	if err != nil {
		return err
	}

	return nil
}
