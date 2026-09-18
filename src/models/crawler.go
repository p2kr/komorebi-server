package models

import (
	"uuid"

	"komorebi-server/src/dto"

	"github.com/Oudwins/zog"
	"gorm.io/gorm"
)

type CrawlerConfig struct {
	Model `tstype:",extends"`

	Key       string `gorm:"not null;uniqueIndex:idx_cc_uniq"`
	Name      string
	Url       string
	IsDeleted bool          `gorm:"not null;uniqueIndex:idx_cc_uniq"`
	Category  dto.MediaType `gorm:"not null;uniqueIndex:idx_cc_uniq"`

	RowSelector        string `gorm:"not null"`
	TitleSelector      string `gorm:"not null"`
	LinkSelector       string `gorm:"not null"`
	PopularitySelector *string
	SizeSelector       *string
}

var ICrawlerConfig = zog.Struct(zog.Shape{
	"Key":           zog.String().Min(1),
	"Url":           zog.String().URL(),
	"Category":      zog.String().OneOf([]string{"Anime", "Manga", "Novel"}),
	"RowSelector":   zog.String().Min(1),
	"TitleSelector": zog.String().Min(1),
	"LinkSelector":  zog.String().Min(1),
})

var ICrawlerResult = zog.Struct(zog.Shape{
	"Title": zog.String().Min(1),
	"Link":  zog.String().URL(),
})

func (c *CrawlerConfig) BeforeSave(tx *gorm.DB) error {
	if c.Id == uuid.Nil() {
		c.Id = uuid.NewV7()
	}

	return nil
}
