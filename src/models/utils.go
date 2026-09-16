package models

import (
	"time"
	"uuid"
)

type Model struct {
	Id        uuid.UUID `gorm:"primarykey;not null"`
	CreatedAt time.Time `gorm:"not null"`
	UpdatedAt time.Time `gorm:"not null"`
}
