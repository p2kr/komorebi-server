package db

import (
	_ "embed"
	"komorebi-server/configs"
	"komorebi-server/src/models"
	"os"
	"path/filepath"

	"github.com/glebarez/sqlite"
	"github.com/rs/zerolog/log"
	"gorm.io/gorm"
)

//go:embed schema.sql
var schema string

var appDb *gorm.DB

func GetDb() *gorm.DB {
	return appDb
}

func SetupDb() {
	config := gorm.Config{} // TODO: Setup logger
	dbPath, _ := filepath.Abs(configs.GetConfig().Db.Path)
	if err := os.MkdirAll(filepath.Dir(dbPath), os.ModePerm); err != nil {
		log.Err(err).Str("dbPath", dbPath).Msg("failed to mkdir")
	}
	db, err := gorm.Open(sqlite.Open(dbPath), &config)
	if err != nil {
		log.Err(err).Str("dbPath", dbPath).Msg("failed to open db")
		db, err = gorm.Open(sqlite.Open(":memory:"), &gorm.Config{})
		if err != nil {
			log.Err(err).Msg("failed to open in memory db")
			panic("failed to open db")
		} else {
			log.Info().Str("dbPath", ":memory:").Msg("Loaded db from in memory")
		}
	} else {
		log.Info().Str("dbPath", dbPath).Msg("Loaded db from dbPath")
	}

	appDb = db

	if configs.GetConfig().Db.ForceCreation {
		DropSchema()
	}

	MigrateSchema()
}

func MigrateSchema() {
	err := appDb.AutoMigrate(&models.User{})
	if err != nil {
		log.Err(err).Msg("Failed to migrate")
	} else {
		log.Info().Msg("Migrated successfully")
	}
}

func DropSchema() {
	err := appDb.Migrator().DropTable(&models.User{})
	if err != nil {
		log.Err(err).Msg("Failed to drop table")
	} else {
		log.Info().Msg("Table dropped successfully")
	}
}
