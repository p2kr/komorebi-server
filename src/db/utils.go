package db

import (
	"context"
	"time"

	"github.com/rs/zerolog/log"
	"gorm.io/gorm/logger"
)

type GormLogger struct{}

func (g *GormLogger) LogMode(level logger.LogLevel) logger.Interface {
	return g
}

func (g *GormLogger) Info(ctx context.Context, msg string, v ...any) {
	log.Ctx(ctx).Info().Msgf(msg, v...)
}

func (g *GormLogger) Warn(ctx context.Context, msg string, v ...any) {
	log.Ctx(ctx).Warn().Msgf(msg, v...)
}

func (g *GormLogger) Error(ctx context.Context, msg string, v ...any) {
	log.Ctx(ctx).Error().Msgf(msg, v...)
}

func (g *GormLogger) Trace(ctx context.Context, begin time.Time, fc func() (sql string, rowsAffected int64), err error) {
	sql, rows := fc()
	log.Ctx(ctx).Debug().
		Dur("elapsed", time.Since(begin)).
		AnErr("error", err).
		Str("sql", sql).
		Int64("rows", rows).
		Send()
}
