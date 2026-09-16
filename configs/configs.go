package configs

import (
	_ "embed"
	"os"
	"strings"
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
	Logger struct {
		LogLevel string
		Pretty   bool
	}
}

func DefaultConfig() (c Config) {
	c.Env.MalClientId = os.Getenv("MAL_CLIENT_ID")
	c.Env.AnilistClientId = os.Getenv("ANILIST_CLIENT_ID")
	c.Env.AppEnv = os.Getenv("APP_ENV")
	if c.Env.AppEnv == "" {
		c.Env.AppEnv = "dev"
	}
	c.Logger.LogLevel = "debug"
	c.Logger.Pretty = true
	return c
}
