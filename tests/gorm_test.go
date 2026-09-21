package tests

import (
	"fmt"
	"testing"
	"time"

	"github.com/glebarez/sqlite"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"
)

type Model struct {
	Id        string    `gorm:"primarykey"`
	CreatedAt time.Time `gorm:"not null"`
	UpdatedAt time.Time `gorm:"not null"`
}

type User struct {
	Model
	Username    string `gorm:"not null;uniqueIndex:idx_uniq"`
	IsSandbox   bool   `gorm:"not null;uniqueIndex:idx_uniq"`
	AccessToken string
}

func TestGorm(t *testing.T) {
	db, err := gorm.Open(sqlite.Open(":memory:"), &gorm.Config{})
	if err != nil {
		panic(err)
	}
	db.AutoMigrate(&User{})

	u1 := User{
		Id: "1", CreatedAt: time.Now(), UpdatedAt: time.Now(),
		Username: "test", IsSandbox: false, AccessToken: "t1",
	}
	db.Create(&u1)
	fmt.Printf("After first insert: id=%s token=%s\n", u1.Id, u1.AccessToken)

	time.Sleep(time.Second)

	u2 := User{
		Id: "2", CreatedAt: time.Now(), UpdatedAt: time.Now(),
		Username: "test", IsSandbox: false, AccessToken: "t2",
	}
	err = db.Clauses(clause.OnConflict{
		Columns:   []clause.Column{{Name: "username"}, {Name: "is_sandbox"}},
		DoUpdates: clause.AssignmentColumns([]string{"access_token", "updated_at"}),
	}, clause.Returning{}).Create(&u2).Error
	if err != nil {
		panic(err)
	}
	fmt.Printf("After upsert: id=%s token=%s err=%v\n", u2.Id, u2.AccessToken, err)
}
