package configs

import (
	_ "embed"
	"os"
	"path/filepath"
	"strings"

	"github.com/pelletier/go-toml/v2"
	"github.com/rs/zerolog/log"
	"github.com/subosito/gotenv"
)

//go:embed config-dev.toml
var DevConfigFS []byte

//go:embed config-prod.toml
var ProdConfigFS []byte

//go:embed config-test.toml
var TestConfigFS []byte

func GetEmbeddedConfig(appEnv string) []byte {
	switch strings.ToLower(strings.TrimSpace(appEnv)) {
	case "dev":
		return DevConfigFS
	case "prod":
		return ProdConfigFS
	case "test":
		return TestConfigFS
	default:
		return DevConfigFS
	}
}

type Config struct {
	Env struct {
		MalClientId     string
		AnilistClientId string
		AppEnv          string
	}
	Db struct {
		Path          string
		ForceCreation bool
	}
	Logger struct {
		LogLevel string
		Pretty   bool
	}
	Frontend struct {
		StaticPath string
	}
}

func getEnv(key string, fallback ...string) string {
	val, isPresent := os.LookupEnv(key)
	if isPresent {
		return val
	} else if len(fallback) > 0 {
		return fallback[0]
	} else {
		return ""
	}
}

func DefaultConfig() (c Config) {
	// Env configs
	c.Env.MalClientId = os.Getenv("MAL_CLIENT_ID")
	c.Env.AnilistClientId = os.Getenv("ANILIST_CLIENT_ID")
	c.Env.AppEnv = getEnv("APP_ENV", "dev")

	// Db configs
	c.Db.Path = getEnv("DB_LOC", "assets/main.sqlite")

	// Logger configs
	c.Logger.LogLevel = "debug"
	c.Logger.Pretty = true

	// Frontend configs
	c.Frontend.StaticPath = "../static"

	return c
}

var appConfig Config

func LoadConfigs() {
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
		config = GetEmbeddedConfig(appEnv)
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
		appConfig = DefaultConfig()
	}

	log.Debug().Msg("Loaded configs")
}

func GetConfig() *Config {
	return &appConfig
}
