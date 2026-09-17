package models

import (
	"time"
	"uuid"
)

type Model struct {
	Id        uuid.UUID `gorm:"primarykey;not null;default=uuid.NewV7()" json:"id"`
	CreatedAt time.Time `gorm:"not null" json:"created_at"`
	UpdatedAt time.Time `gorm:"not null" json:"updated_at"`
}
