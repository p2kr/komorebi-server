package processors

import (
	"context"
	"errors"

	"komorebi-server/src/models"
)

type Processor interface {
	Process(ctx context.Context, item models.VaultItem, outputDir string) error
}

var RemuxProcessor = remux{}

func ProcessVideo(ctx context.Context, item models.VaultItem, outputDir string) error {
	if item.FilePath == "" {
		return errors.New("invalid file path")
	}
	return RemuxProcessor.Process(ctx, item, outputDir)
}
