package models

type User struct {
	Model

	Username    string `gorm:"not null;uniqueIndex:idx_uniq"`
	ProviderId  *string
	AvatarUrl   *string
	Provider    Provider `gorm:"not null;uniqueIndex:idx_uniq"`
	IsSandbox   bool     `gorm:"not null;uniqueIndex:idx_uniq"`
	AccessToken *string
	Passcode    *string
}
