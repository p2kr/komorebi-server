package workers

import (
	"context"
	"io/fs"
	"os"
	"path/filepath"
	"strings"

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

func identifyFile(path string) (models.VaultItem, error) {
	log := zlog.With().Str("path", lo.Substring(path, 0, 25)).Logger()

	// Should I remove extension?
	name := strings.TrimSuffix(filepath.Base(path), filepath.Ext(path))
	item := models.VaultItem{
		FileName: name,
		FilePath: path,
	}

	//  Invoke ffprobe
	probe, err := ffmpeg.Probe(path, ffmpeg.KwArgs{
		"show_chapters": "",
	})
	if err != nil {
		log.Debug().Err(err).Msg("Failed to probe file")
		return item, err
	}

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
			audioTracks = append(audioTracks, models.AudioTrack{
				StreamIdx: index,
				IsDefault: result.Get("disposition.default").Bool(),
				Codec:     result.Get("codec_name").String(),
				Channels:  result.Get("channels").Int(),
				Lang:      result.Get("tags.language").String(),
				Title:     result.Get("tags.title").String(),
			})
			continue
		}

		// Identify Subtitles
		if result.Get("codec_type").String() == "subtitle" {
			subtitles = append(subtitles, models.VideoSubtitle{
				StreamIdx: index,
				IsForced: result.Get("disposition.default").Bool() ||
					result.Get("disposition.forced").Bool(),
				Format: result.Get("codec_name").String(),
				Lang:   result.Get("tags.language").String(),
				Title:  result.Get("tags.title").String(),
			})
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
