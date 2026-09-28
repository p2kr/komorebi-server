package workers

import (
	"context"
	"errors"
	"io/fs"
	"os"
	"os/signal"
	"path/filepath"
	"strings"

	"golang.org/x/sync/errgroup"
	"golang.org/x/text/language"
	"golang.org/x/text/language/display"

	"github.com/cenkalti/backoff/v7"

	"komorebi-server/src/processors"

	"github.com/go-co-op/gocron/v2"

	"komorebi-server/src/dto"

	"komorebi-server/src/db"

	"komorebi-server/src/models"

	mapset "github.com/deckarep/golang-set/v3"
	zlog "github.com/rs/zerolog/log"
	"github.com/samber/lo"
	"github.com/tidwall/gjson"
	ffmpeg "github.com/u2takey/ffmpeg-go"
)

// PostDownload is a worker function that handles post-download operations.
// It identifies each file in the vault and creates a metadata entry in db for it.
func PostDownload(jobs ...models.DownloadJob) {
	if len(jobs) == 0 {
		return
	}

	var vaultItems []models.VaultItem
	for i := range len(jobs) {
		zlog.Debug().Any("job id", jobs[i].Id).Msg("Post Processing download job")
		items := postDownloadOne(&jobs[i])
		if len(items) > 0 {
			vaultItems = append(vaultItems, items...)
		}
	}

	ctx := context.Background()
	// Save to db
	db.SaveVaultItems(ctx, vaultItems...)

	// Remux them in a background job
	ScheduleRemuxJob(vaultItems...)
}

func postDownloadOne(job *models.DownloadJob) []models.VaultItem {
	if job == nil {
		return nil
	}

	fsys := os.DirFS(job.Location)

	videoExtensions := mapset.NewSet(".mp4", ".mkv", ".webm")

	var vaultItems []models.VaultItem

	fs.WalkDir(fsys, ".", func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return nil
		}
		if d.IsDir() { // Don't process directories
			if d.Name() == "temp" { // Skip temp directories
				return fs.SkipDir
			}
			return nil
		}

		info, _ := d.Info()

		ext := strings.ToLower(filepath.Ext(info.Name()))
		if !videoExtensions.ContainsOne(ext) {
			return nil
		}

		item, err := identifyFile(filepath.Join(job.Location, path))
		if err != nil {
			return nil
		}
		item.DownloadJobId = job.Id
		vaultItems = append(vaultItems, item)
		return nil
	})

	return vaultItems
}

func resolveLanguage(langCode string) string {
	if langCode == "" {
		return ""
	}
	tag, err := language.Parse(langCode)
	if err == nil {
		if parsedName := display.English.Tags().Name(tag); parsedName != "" {
			return parsedName
		}
	}
	return langCode
}

func identifyFile(path string) (models.VaultItem, error) {
	log := zlog.With().Str("path", lo.Substring(path, 0, 25)).Logger()

	// Should I remove extension?
	name := strings.TrimSuffix(filepath.Base(path), filepath.Ext(path))
	item := models.VaultItem{
		FileName: name,
		FilePath: path,
		Status:   models.DownloadStatusProcessing,
	}

	//  Invoke ffprobe
	probe, err := ffmpeg.Probe(path, ffmpeg.KwArgs{
		"show_chapters": "",
	})
	if err != nil {
		log.Debug().Err(err).Msg("Failed to probe file")
		return item, err
	}

	item.Format = gjson.Get(probe, "format.format_name").String()
	item.SizeInBytes = gjson.Get(probe, "format.size").Int()
	item.DurationSec = gjson.Get(probe, "format.duration").Float()
	if item.DurationSec == 0 {
		return item, nil
	}

	item.FileType = dto.MediaTypeAnime

	var chapters []models.VideoChapter
	var videoTracks []models.VideoTrack
	var audioTracks []models.AudioTrack
	var subtitles []models.VideoSubtitle
	var fonts []models.SubtitleFont

	probeChapters := gjson.Get(probe, "chapters").Array()
	streams := gjson.Get(probe, "streams").Array()

	for _, result := range probeChapters {
		chapters = append(chapters, models.VideoChapter{
			StreamIdx: result.Get("id").Int(),
			StartTime: result.Get("start_time").Float(),
			EndTime:   result.Get("end_time").Float(),
			Title:     result.Get("tags.title").String(),
		})
	}

	for _, result := range streams {
		index := result.Get("index").Int()

		// Identify video stream
		if result.Get("codec_type").String() == "video" {
			videoTracks = append(videoTracks, models.VideoTrack{
				StreamIdx: index,
				IsPrimary: result.Get("disposition.default").Bool(),
				Codec:     result.Get("codec_name").String(),
				PixFmt:    result.Get("pix_fmt").String(),
			})
			continue
		}

		// Identify audio stream
		if result.Get("codec_type").String() == "audio" {
			lang := result.Get("tags.language").String()
			title := result.Get("tags.title").String()
			if title == "" {
				title = resolveLanguage(lang)
			}

			audioTracks = append(audioTracks, models.AudioTrack{
				StreamIdx: index,
				IsDefault: result.Get("disposition.default").Bool(),
				Codec:     result.Get("codec_name").String(),
				Channels:  result.Get("channels").Int(),
				Lang:      lang,
				Title:     title,
			})
			continue
		}

		// Identify Subtitles
		if result.Get("codec_type").String() == "subtitle" {
			codec := strings.ToLower(result.Get("codec_name").String())
			ext := processors.AllowedSubtitles[codec]
			if ext != "" {
				lang := result.Get("tags.language").String()
				title := result.Get("tags.title").String()
				if title == "" {
					title = resolveLanguage(lang)
				}

				subtitles = append(subtitles, models.VideoSubtitle{
					StreamIdx: index,
					IsForced: result.Get("disposition.default").Bool() ||
						result.Get("disposition.forced").Bool(),
					Format: ext,
					Lang:   lang,
					Title:  title,
				})
			} else {
				log.Debug().Str("codec_name", codec).Msg("Skipping subtitle")
			}
			continue
		}

		// Identify Fonts
		if result.Get("codec_type").String() == "attachment" {
			fonts = append(fonts, models.SubtitleFont{
				StreamIdx: index,
				Format:    result.Get("codec_name").String(),
				FontName:  result.Get("tags.filename").String(),
			})
			continue
		}
	}

	item.VideoChapters = chapters
	item.VideoTracks = videoTracks
	item.AudioTracks = audioTracks
	item.VideoSubtitles = subtitles
	item.SubtitleFonts = fonts

	return item, nil
}

func ScheduleRemuxJob(items ...models.VaultItem) {
	task := gocron.NewTask(func(givenItems []models.VaultItem) {
		// Tie context to application lifecycle so ffmpeg dies on graceful shutdown (Ctrl+C)
		ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt)
		defer cancel()

		// Delete original files that failed to delete previously in a background goroutine
		go func() {
			var readyItems []models.VaultItem
			db.GetDb().Where("status = ?", models.DownloadStatusReady).Find(&readyItems)
			for _, rItem := range readyItems {
				if _, err := os.Stat(rItem.FilePath); err == nil {
					_, _ = backoff.Retry(context.Background(), func() (any, error) {
						err := os.Remove(rItem.FilePath)
						if err != nil && !errors.Is(err, os.ErrNotExist) {
							return nil, err
						}
						return nil, nil
					}, backoff.WithMaxTries(5))
				}
			}
		}()

		var targets []models.VaultItem

		if len(givenItems) > 0 {
			targets = givenItems
		} else {
			db.GetDb().Preload("VideoTracks").Preload("AudioTracks").Preload("VideoSubtitles").
				Where("status != ?", models.DownloadStatusReady).Find(&targets)
		}
		zlog.Debug().Int("target size", len(targets)).Msg("post processing")

		if len(targets) == 0 {
			return
		}

		eg, egCtx := errgroup.WithContext(ctx)
		eg.SetLimit(2) // Max 2 concurrent FFmpeg jobs
		for i := range targets {
			eg.Go(func() error {
				processors.PostProcessor.ProcessOne(egCtx, &targets[i])
				return nil
			})
		}
		eg.Wait()
		db.SaveVaultItems(ctx, targets...)
	}, items)

	_, err := AddOneTimeJob(task)
	if err != nil {
		zlog.Err(err).Msg("Failed to enqueue remux job")
	}
}
