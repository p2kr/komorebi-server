package core

import (
	"komorebi-server/configs"
	"os"
	"path/filepath"
	"strings"

	"github.com/pelletier/go-toml/v2"
	"github.com/rs/zerolog/log"

	"github.com/subosito/gotenv"
)

var appConfig configs.Config

// Init Initializes all configuration, including reading from env.
func Init() {
	// Read config
	loadConfigs()

	// Setup Logger
	SetupLogger(appConfig)

	log.Info().Msg("Initialized App")
}

func GetAppConfig() configs.Config {
	return appConfig
}

func loadConfigs() {
	gotenv.Load(".env")

	appEnv, isPresent := os.LookupEnv("APP_ENV")
	if !isPresent {
		log.Warn().Msg("APP_ENV not found. Defaulting to 'dev'")
		appEnv = "dev"
	}

	fileName := filepath.Join("configs", "config-"+appEnv+".toml")
	config, err := os.ReadFile(fileName)
	if err != nil {
		log.Err(err).Str("fileName", fileName).Msg("failed to read config from app env")
		config = configs.GetEmbeddedConfig(appEnv)
	}

	log.Info().Str("fileName", fileName).Msg("Loading [fileName] for env")

	configStr := string(config)
	configStr = os.Expand(configStr, func(s string) string {
		parts := strings.SplitN(s, ":", 2)
		key := parts[0]
		if val, exists := os.LookupEnv(key); exists {
			log.Debug().Str("ENV", key).Msg("exists")
			return val
		}
		// If the variable doesn't exist and a default was provided, use it
		if len(parts) > 1 {
			log.Debug().Str("ENV", key).Str("default", parts[1]).Msg("does not exist. Returning default")
			return parts[1]
		}
		log.Debug().Str("ENV", key).Msg("does not exist and no default provided")
		return ""
	})

	config = []byte(configStr)

	// Find not embedded file
	if err = toml.Unmarshal(config, &appConfig); err != nil {
		log.Err(err).Bytes("config", config).Msg("failed to unmarshal")
		appConfig = configs.DefaultConfig()
	}

	log.Debug().Msg("Loaded configs")
}
