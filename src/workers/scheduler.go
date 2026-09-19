package workers

import (
	"context"
	"uuid"

	guuid "github.com/google/uuid"

	"github.com/go-co-op/gocron/v2"
	"github.com/rs/zerolog/log"
)

var scheduler gocron.Scheduler

func InitScheduler() {
	// create scheduler
	var err error
	scheduler, err = gocron.NewScheduler()
	if err != nil {
		log.Err(err).Msg("failed to initialize scheduler")
		return
	}

	AddJobs(&scheduler)

	// start the scheduler
	log.Info().Msg("Started Scheduler")
	scheduler.Start()
}

func CloseScheduler() {
	err := scheduler.Shutdown()
	if err != nil {
		log.Err(err).Msg("Failed to shutdown scheduler")
	}
	log.Info().Msg("Scheduler shutdown complete")
}

func AddJob(jd gocron.JobDefinition, task gocron.Task, opts ...gocron.JobOption) (gocron.Job, error) {
	job, err := scheduler.NewJob(jd, task, opts...)
	log.Debug().Err(err).Any("ID", job.ID()).Any("schedule", job.Schedule()).Msg("Job scheduled")
	return job, err
}

func AddJobWithId(jd gocron.JobDefinition, task gocron.Task, id uuid.UUID, opts ...gocron.JobOption) (gocron.Job, error) {
	allOpts := append([]gocron.JobOption{gocron.WithIdentifier(guuid.UUID(id))}, opts...)
	return scheduler.NewJob(jd, task, allOpts...)
}

func RemoveJob(id uuid.UUID) error {
	log.Debug().Any("ID", id).Msg("Job removed from scheduler")
	return scheduler.RemoveJob(guuid.UUID(id))
}

type Job interface {
	Run(ctx context.Context, args ...any)
}

func AddJobs(s *gocron.Scheduler) {
	// Add Jobs here
}
