package tests

import (
	"bytes"
	"testing"

	"komorebi-server/src/db"
	"komorebi-server/src/models"

	"github.com/glebarez/sqlite"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
	"gorm.io/gorm"
	"gorm.io/gorm/logger"
)

func TestGormLoggerInterface(t *testing.T) {
	var _ logger.Interface = &db.GormLogger{}
}

func TestGormLoggerOutput(t *testing.T) {
	var buf bytes.Buffer
	origLogger := log.Logger
	defer func() {
		log.Logger = origLogger
		zerolog.DefaultContextLogger = &origLogger
	}()

	memLogger := zerolog.New(&buf).Level(zerolog.DebugLevel)
	log.Logger = memLogger
	zerolog.DefaultContextLogger = &memLogger

	dbConn, err := gorm.Open(sqlite.Open(":memory:"), &gorm.Config{
		Logger: &db.GormLogger{},
	})
	require.NoError(t, err, "failed to open sqlite memory db")

	err = dbConn.AutoMigrate(&models.User{})
	require.NoError(t, err, "failed to migrate")

	output := buf.String()
	assert.Contains(t, output, "sql", "expected output to contain 'sql'")
	assert.Contains(t, output, "elapsed", "expected output to contain 'elapsed'")
}

func TestCrawlerConfigIndex(t *testing.T) {
	dbConn, err := gorm.Open(sqlite.Open(":memory:"), &gorm.Config{})
	require.NoError(t, err, "failed to open sqlite memory db")

	err = dbConn.AutoMigrate(&models.CrawlerConfig{})
	require.NoError(t, err, "failed to migrate CrawlerConfig")

	hasIndex := dbConn.Migrator().HasIndex(&models.CrawlerConfig{}, "idx_cc_uniq")
	assert.True(t, hasIndex, "expected index idx_cc_uniq to be created")
}
