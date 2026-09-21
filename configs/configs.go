package configs

import (
	_ "embed"
	"os"
	"path/filepath"
	"strings"

	"github.com/rs/zerolog/log"
	"github.com/subosito/gotenv"

	"github.com/knadh/koanf/parsers/toml"
	"github.com/knadh/koanf/providers/confmap"
	"github.com/knadh/koanf/providers/env"
	"github.com/knadh/koanf/providers/file"
	"github.com/knadh/koanf/providers/rawbytes"
	"github.com/knadh/koanf/v2"
)

//go:embed config-dev.toml
var DevConfigFS []byte

//go:embed config-prod.toml
var ProdConfigFS []byte

//go:embed config-test.toml
var TestConfigFS []byte

//go:embed seed.toml
var DbSeed []byte

func getEmbeddedConfig(appEnv string) []byte {
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

// Config holds all application configuration. Struct tags ensure koanf maps
// keys deterministically regardless of case during Unmarshal.
type Config struct {
	Env struct {
		MalClientId           string `koanf:"malClientId"`
		AnilistClientId       string `koanf:"anilistClientId"`
		AppEnv                string `koanf:"appEnv"`
		DefaultHostedAuthPage string `koanf:"defaultHostedAuthPage"`
		VaultLoc              string `koanf:"vaultLoc"`
	} `koanf:"env"`
	Db struct {
		Path          string `koanf:"path"`
		ForceCreation bool   `koanf:"forceCreation"`
		Debug         bool   `koanf:"debug"`
	} `koanf:"db"`
	Logger struct {
		LogLevel      string `koanf:"logLevel"`
		Pretty        bool   `koanf:"pretty"`
		PrintConfig   bool   `koanf:"printConfig"`
		PrintCrawling bool   `koanf:"printCrawling"`
	} `koanf:"logger"`
	Frontend struct {
		StaticPath string `koanf:"staticPath"`
	} `koanf:"frontend"`
	HttpClient struct {
		Debug          bool   `koanf:"debug"`
		CurlCmd        bool   `koanf:"curlCmd"`
		CrawlerTimeout string `koanf:"crawlerTimeoutSec"`
	} `koanf:"httpClient"`
	TorrentClient struct {
		Debug bool `koanf:"debug"`
	} `koanf:"torrentClient"`
}

var appConfig *Config

func GetConfig() *Config {
	return appConfig
}

func LoadConfigs() {
	k := koanf.New(".") // "." is the key delimiter: "db.path", "env.appEnv"

	// -- Layer 1: Hardcoded Go defaults (lowest priority) ------------------
	// Keys MUST use the same camelCase as the TOML files and the env mapping
	// below. Koanf''s internal map is case-sensitive: mismatched casing creates
	// duplicate keys and causes non-deterministic Unmarshal results.
	k.Load(confmap.Provider(map[string]any{
		"env.appEnv":                   "dev",
		"env.defaultHostedAuthPage":    "https://p2kr.github.io/komorebi-web/auth.html",
		"env.vaultLoc":                 "vault",
		"db.path":                      "assets/main.sqlite",
		"logger.logLevel":              "debug",
		"logger.pretty":                false,
		"frontend.staticPath":          "../static",
		"httpClient.crawlerTimeoutSec": "60s",
	}, "."), nil)

	// -- Layer 2: .env file -> pushed into OS env so the steps below can read it
	// Use Load (not OverLoad) so existing shell env vars are not clobbered.
	if err := gotenv.Load(".env"); err != nil {
		log.Warn().Err(err).Msg(".env file not found, skipping")
	}

	// -- Resolve APP_ENV from OS env (populated by .env above or the shell) --
	// We read from os.Getenv here because gotenv.Load writes to the OS
	// environment, not into koanf. The koanf env provider runs last (Layer 5),
	// so k.String("env.appEnv") would still return the Layer 1 default here.
	appEnv := os.Getenv("APP_ENV")
	if appEnv == "" {
		appEnv = k.String("env.appEnv") // fall back to hardcoded default ("dev")
	}
	appEnv = strings.ToLower(strings.TrimSpace(appEnv))

	// -- Layer 3: Embedded TOML for the resolved environment ---------------
	embeddedCfg := getEmbeddedConfig(appEnv)
	if err := k.Load(rawbytes.Provider(embeddedCfg), toml.Parser()); err != nil {
		log.Err(err).Str("appEnv", appEnv).Msg("failed to parse embedded TOML config")
	} else {
		log.Debug().Str("appEnv", appEnv).Msg("Loaded embedded config")
	}

	// -- Layer 4: Filesystem TOML (overrides embedded, for local dev) ------
	// When the configs/ directory is present on disk, the file-based TOML
	// takes precedence over the embedded bytes - useful for local overrides
	// without recompiling.
	fileName := filepath.Join("configs", "config-"+appEnv+".toml")
	if err := k.Load(file.Provider(fileName), toml.Parser()); err != nil {
		log.Warn().Str("file", fileName).Msg("No filesystem config found, using embedded")
	} else {
		log.Info().Str("file", fileName).Msg("Loaded config from filesystem")
	}

	// -- Layer 5: Environment variables (highest priority) -----------------
	// Secrets and deployment-specific values always win over any TOML file.
	// The callback maps exact OS env var names to their koanf key paths.
	// Returning "" for an unrecognized var causes koanf to skip it entirely.
	if err := k.Load(env.Provider("", ".", func(s string) string {
		m := map[string]string{
			"MAL_CLIENT_ID":            "env.malClientId",
			"ANILIST_CLIENT_ID":        "env.anilistClientId",
			"APP_ENV":                  "env.appEnv",
			"DEFAULT_HOSTED_AUTH_PAGE": "env.defaultHostedAuthPage",
			"VAULT_LOC":                "env.vaultLoc",
			"DB_LOC":                   "db.path",
		}
		return m[s] // returns "" (skip) for any key not in the map
	}), nil); err != nil {
		log.Warn().Err(err).Msg("Failed to load env variables")
	}

	// -- Unmarshal into struct ----------------------------------------------
	appConfig = &Config{}
	if err := k.Unmarshal("", appConfig); err != nil {
		log.Fatal().Err(err).Msg("Failed to unmarshal config - cannot start")
	}

	if appConfig.Logger.PrintConfig && appConfig.Env.AppEnv == "dev" {
		log.Debug().Any("config", appConfig).Msg("Loaded config")
	}
}
