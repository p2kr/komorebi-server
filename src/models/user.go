package models

import (
	"encoding/json"
	"komorebi-server/src/dto"
	"uuid"

	"gorm.io/gorm"
	"gorm.io/gorm/clause"
)

type User struct {
	Model

	Username    string            `gorm:"not null;uniqueIndex:idx_uniq" json:"username"`
	ProviderId  *string           `json:"provider_id"`
	AvatarUrl   *string           `json:"avatar_url"`
	Provider    dto.MediaProvider `gorm:"not null;uniqueIndex:idx_uniq" json:"provider"`
	IsSandbox   bool              `gorm:"not null;uniqueIndex:idx_uniq" json:"is_sandbox"`
	AccessToken *string           `json:"access_token,omitempty"`
	Passcode    *string           `json:"passcode,omitempty"`
}

// Hide [Passcode] and [AccessToken]
func (u User) MarshalJSON() ([]byte, error) {
	type Alias User

	a := Alias(u)
	a.Passcode = nil
	a.AccessToken = nil

	return json.Marshal(a)
}

func (User) TableOptions() string {
	return "STRICT"
}

func (u *User) BeforeSave(tx *gorm.DB) error {
	if u.Id == uuid.Nil() {
		u.Id = uuid.NewV7()
	}

	if u.AccessToken == nil || *u.AccessToken == "" {
		u.IsSandbox = true
	} else {
		u.IsSandbox = false
	}

	return nil
}

func (u *User) InsertOrUpdate(tx *gorm.DB) error {
	err := tx.Clauses(clause.OnConflict{
		Columns:   []clause.Column{{Name: "username"}, {Name: "provider"}, {Name: "is_sandbox"}},
		DoUpdates: clause.AssignmentColumns([]string{"provider_id", "access_token", "is_sandbox", "avatar_url", "updated_at"}),
	}, clause.Returning{}).Create(u).Error

	return err
}
