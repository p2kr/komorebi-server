package workers

import (
	"github.com/go-co-op/gocron/v2"
	"github.com/rs/zerolog/log"
)

var scheduler gocron.Scheduler

func GetScheduler() *gocron.Scheduler {
	return &scheduler
}

func InitScheduler() {
	// create scheduler
	s, err := gocron.NewScheduler()
	if err != nil {
		log.Err(err).Msg("failed to initialize scheduler")
		return
	}

	scheduler = s
}

func AddJob(jd gocron.JobDefinition, task gocron.Task, opts ...gocron.JobOption) (gocron.Job, error) {
	j, err := scheduler.NewJob(
		jd, task, opts...,
	)

	if err != nil {
		return j, err
	}

	return j, nil
}
