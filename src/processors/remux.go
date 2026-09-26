package processors

import (
	"bytes"
	"context"
	"io"

	"komorebi-server/src/models"

	zlog "github.com/rs/zerolog/log"
	ffmpeg "github.com/u2takey/ffmpeg-go"
)

type remux struct{}

func (r *remux) Process(ctx context.Context, item models.VaultItem, seek float64, writer io.Writer) error {
	input := item.FilePath

	stream := ffmpeg.Input(input, ffmpeg.KwArgs{
		"ss": seek,
	}).Output("pipe:1", ffmpeg.KwArgs{
		"format": "mp4",
		//"movflags": "frag_keyframe+empty_moov+default_base_moof",
		"movflags": "frag_keyframe+delay_moov+default_base_moof",
		"map":      []string{"0:v?", "0:a?", "0:s?"},
		"c:v":      "copy",
		"c:a":      "copy",
		"c:s":      "mov_text",
		//"c:s": "webvtt",
	})
	stream.Context = ctx

	var stderr bytes.Buffer
	stream.
		WithOutput(writer).OverWriteOutput().
		WithErrorOutput(&stderr).
		GlobalArgs("-progress", "-v error")

	zlog.Debug().Str("file", input).Strs("command", stream.GetArgs()).Msg("ffmpeg running")

	err := stream.Run()

	zlog.Err(err).Str("file", input).Str("output", stderr.String()).Msg("ffmpeg done")
	if err != nil {
		return err
	}

	return nil
}
