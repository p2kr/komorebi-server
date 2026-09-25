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
	VaultItemId uuid.UUID `json:"vault_item_id,omitempty"`
	FilePath    string    `json:"file_path,omitempty"`
	StreamIdx   int64     `json:"stream_idx,omitempty"`
}

func (v *Model) BeforeSave(tx *gorm.DB) error {
	if v.Id == uuid.Nil() {
		v.Id = uuid.NewV7()
	}
	return nil
}
