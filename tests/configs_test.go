package tests

import (
	"komorebi-server/configs"
	"os"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestLoadConfigs_Defaults(t *testing.T) {
	// Clean env so we test pure defaults
	os.Unsetenv("APP_ENV")
	os.Unsetenv("MAL_CLIENT_ID")
	os.Unsetenv("ANILIST_CLIENT_ID")
	os.Unsetenv("DB_LOC")
	os.Unsetenv("DEFAULT_HOSTED_AUTH_PAGE")

	configs.LoadConfigs()
	c := configs.GetConfig()

	assert.NotNil(t, c, "config should not be nil")

	// From TOML dev defaults
	assert.Equal(t, "dev", c.Env.AppEnv, "AppEnv should default to dev")
	assert.Equal(t, "https://p2kr.github.io/komorebi-web/auth.html", c.Env.DefaultHostedAuthPage)
	assert.Equal(t, "assets/main.sqlite?mode=rwc", c.Db.Path)
	assert.Equal(t, "debug", c.Logger.LogLevel)
	assert.Equal(t, true, c.Logger.Pretty)
	assert.Equal(t, false, c.Db.ForceCreation)

	// Secrets should be empty when env vars not set
	assert.Equal(t, "", c.Env.MalClientId, "MalClientId should be empty when env var not set")
	assert.Equal(t, "", c.Env.AnilistClientId, "AnilistClientId should be empty when env var not set")
}

func TestLoadConfigs_EnvOverride(t *testing.T) {
	os.Setenv("MAL_CLIENT_ID", "test-mal-id")
	os.Setenv("ANILIST_CLIENT_ID", "test-anilist-id")
	os.Setenv("DB_LOC", "/tmp/override.sqlite")
	defer func() {
		os.Unsetenv("MAL_CLIENT_ID")
		os.Unsetenv("ANILIST_CLIENT_ID")
		os.Unsetenv("DB_LOC")
	}()

	configs.LoadConfigs()
	c := configs.GetConfig()

	assert.Equal(t, "test-mal-id", c.Env.MalClientId, "MAL_CLIENT_ID env var should override config")
	assert.Equal(t, "test-anilist-id", c.Env.AnilistClientId, "ANILIST_CLIENT_ID env var should override config")
	assert.Equal(t, "/tmp/override.sqlite", c.Db.Path, "DB_LOC env var should override db.path")
}
