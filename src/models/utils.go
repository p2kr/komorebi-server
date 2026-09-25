package models

import (
	"time"
	"uuid"

	"gorm.io/gorm"
)

type Model struct {
	Id        uuid.UUID `gorm:"primarykey;not null" json:"id"`
	CreatedAt time.Time `gorm:"not null" json:"created_at"`
	UpdatedAt time.Time `gorm:"not null" json:"updated_at"`
}

type VaultModel struct {
	Model       `tstype:",extends"`
	VaultItemId uuid.UUID
	FilePath    string
	StreamIdx   int64
}

func (v *Model) BeforeSave(tx *gorm.DB) error {
	if v.Id == uuid.Nil() {
		v.Id = uuid.NewV7()
	}
	return nil
}
