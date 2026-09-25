package models

import (
	"errors"
	"strings"
	"uuid"

	"gorm.io/gorm"
)

type VaultItem struct {
	Model `tstype:",extends"`

	DownloadJobId uuid.UUID

	FileName string
	FilePath string

	DurationSec float64
	SizeInBytes int64

	VideoChapters  []VideoChapter  `gorm:"constraint:OnDelete:CASCADE;"`
	VideoSubtitles []VideoSubtitle `gorm:"constraint:OnDelete:CASCADE;"`
	SubtitleFonts  []SubtitleFont  `gorm:"constraint:OnDelete:CASCADE;"`
	AudioTracks    []AudioTrack    `gorm:"constraint:OnDelete:CASCADE;"`
	VideoTracks    []VideoTrack    `gorm:"constraint:OnDelete:CASCADE;"`
}

func (v *VaultItem) BeforeSave(tx *gorm.DB) error {
	if v.Id == uuid.Nil() {
		v.Id = uuid.New()
	}

	if strings.TrimSpace(v.FileName) == "" {
		return errors.New("file name cannot be empty")
	}

	return nil
}

type VideoChapter struct {
	VaultModel `tstype:",extends"`
	ChapterId  int
	Title      string
	StartTime  float64
	EndTime    float64
}

type VideoSubtitle struct {
	VaultModel `tstype:",extends"`
	Track      int
	Lang       string
	Title      string
	Format     string
	IsForced   bool
}

type VideoTrack struct {
	VaultModel `tstype:",extends"`
	Codec      string
	PixFmt     string
	IsPrimary  bool
}

type AudioTrack struct {
	VaultModel `tstype:",extends"`
	Lang       string
	Title      string
	Channels   int64
	Codec      string
	IsDefault  bool
}

type SubtitleFont struct {
	VaultModel `tstype:",extends"`
	FontName   string
	Format     string
}
