package models

import (
	"errors"
	"strings"
	"uuid"

	"komorebi-server/src/dto"

	"gorm.io/gorm"
)

type VaultItem struct {
	Model `tstype:",extends"`

	DownloadJobId uuid.UUID `json:"download_job_id"`

	FileName string        `json:"file_name"`
	FilePath string        `json:"file_path"`
	FileType dto.MediaType `json:"file_type,omitempty"`

	Format      string  `json:"format,omitempty"`
	DurationSec float64 `json:"duration_sec,omitempty"`
	SizeInBytes int64   `json:"size_in_bytes"`

	VideoChapters  []VideoChapter  `gorm:"constraint:OnDelete:CASCADE;" json:"video_chapters,omitempty"`
	VideoSubtitles []VideoSubtitle `gorm:"constraint:OnDelete:CASCADE;" json:"video_subtitles,omitempty"`
	SubtitleFonts  []SubtitleFont  `gorm:"constraint:OnDelete:CASCADE;" json:"subtitle_fonts,omitempty"`
	AudioTracks    []AudioTrack    `gorm:"constraint:OnDelete:CASCADE;" json:"audio_tracks,omitempty"`
	VideoTracks    []VideoTrack    `gorm:"constraint:OnDelete:CASCADE;" json:"video_tracks,omitempty"`
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
	ChapterId  int     `json:"chapter_id,omitempty"`
	Title      string  `json:"title,omitempty"`
	StartTime  float64 `json:"start_time,omitempty"`
	EndTime    float64 `json:"end_time,omitempty"`
}

type VideoSubtitle struct {
	VaultModel `tstype:",extends"`
	Track      int    `json:"track,omitempty"`
	Lang       string `json:"lang,omitempty"`
	Title      string `json:"title,omitempty"`
	Format     string `json:"format,omitempty"`
	IsForced   bool   `json:"is_forced,omitempty"`
}

type VideoTrack struct {
	VaultModel `tstype:",extends"`
	Codec      string `json:"codec,omitempty"`
	PixFmt     string `json:"pix_fmt,omitempty"`
	IsPrimary  bool   `json:"is_primary,omitempty"`
}

type AudioTrack struct {
	VaultModel `tstype:",extends"`
	Lang       string `json:"lang,omitempty"`
	Title      string `json:"title,omitempty"`
	Channels   int64  `json:"channels,omitempty"`
	Codec      string `json:"codec,omitempty"`
	IsDefault  bool   `json:"is_default,omitempty"`
}

type SubtitleFont struct {
	VaultModel `tstype:",extends"`
	FontName   string `json:"font_name,omitempty"`
	Format     string `json:"format,omitempty"`
}
