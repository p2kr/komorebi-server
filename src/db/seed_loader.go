package db

import (
	"komorebi-server/configs"

	"github.com/pelletier/go-toml/v2"
	"github.com/rs/zerolog/log"
)

func LoadSeedsInDb() {
	// Parse seed.toml
	var seed map[string]any
	err := toml.Unmarshal(configs.DbSeed, &seed)
	if err != nil {
		log.Err(err).Bytes("seed", configs.DbSeed).Msg("Failed to unmarshall db seed")
		return
	}

	// Insert into db
}
