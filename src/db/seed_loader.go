package db

import (
	"komorebi-server/configs"
	"komorebi-server/src/models"

	"github.com/pelletier/go-toml/v2"
	"github.com/rs/zerolog/log"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"
)

type Seed struct {
	CrawlerConfig []models.CrawlerConfig `toml:"crawler_config"`
}

func LoadSeedsInDb(db *gorm.DB) {
	// Parse seed.toml
	var seed Seed
	err := toml.Unmarshal(configs.DbSeed, &seed)
	if err != nil {
		log.Err(err).Bytes("seed", configs.DbSeed).Msg("Failed to unmarshall db seed")
		return
	}

	err = InsertOrUpdateCrawlerConfigs(db, &seed.CrawlerConfig)
	if err != nil {
		log.Err(err).Any("config", seed.CrawlerConfig).Msg("failed to insert crawler config in db")
	} else {
		log.Info().Int("count", len(seed.CrawlerConfig)).Msgf("Successfully inserted crawler config into db")
	}
}

func InsertOrUpdateCrawlerConfigs(db *gorm.DB, c *[]models.CrawlerConfig) error {
	return db.Clauses(
		clause.OnConflict{
			Columns:   []clause.Column{{Name: "key"}, {Name: "is_deleted"}, {Name: "category"}},
			DoNothing: true,
		},
	).CreateInBatches(c, len(*c)).Error
}
