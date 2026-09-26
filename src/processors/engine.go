package processors

import (
	"context"
	"errors"
	"io"

	"komorebi-server/src/models"
)

type Processor interface {
	Process(ctx context.Context, item models.VaultItem, seek float64, writer io.Writer) error
}

var remuxProcessor = remux{}

func ProcessVideo(ctx context.Context, item models.VaultItem, seek float64, writer io.Writer) error {
	if item.FilePath == "" {
		return errors.New("invalid file path")
	}
	return remuxProcessor.Process(ctx, item, seek, writer)
}
