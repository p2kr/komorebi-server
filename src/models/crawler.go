package models

import (
	"komorebi-server/src/dto"

	"github.com/Oudwins/zog"
)

type CrawlerConfig struct {
	Model

	Key       string        `gorm:"not null;uniqueIndex=idx_cc_uniq" json:"key"`
	Name      string        `json:"name"`
	Url       string        `json:"url"`
	IsDeleted bool          `gorm:"not null;uniqueIndex=idx_cc_uniq" json:"is_deleted"`
	Category  dto.MediaType `gorm:"not null;uniqueIndex=idx_cc_uniq" json:"category"`

	RowSelector        string  `gorm:"not null" json:"row_selector"`
	TitleSelector      string  `gorm:"not null" json:"title_selector"`
	LinkSelector       string  `gorm:"not null" json:"link_selector"`
	PopularitySelector *string `json:"popularity_selector"`
	SizeSelector       *string `json:"size_selector"`
}

var ICrawlerConfig = zog.Struct(zog.Shape{
	"Key":           zog.String().Min(1),
	"Url":           zog.String().URL(),
	"Category":      zog.String().OneOf([]string{"Anime", "Manga", "Novel"}),
	"RowSelector":   zog.String().Min(1),
	"TitleSelector": zog.String().Min(1),
	"LinkSelector":  zog.String().Min(1),
})
