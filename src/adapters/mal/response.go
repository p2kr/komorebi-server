package mal

import (
	"errors"
	"strconv"
	"strings"
	"uuid"

	"komorebi-server/src/dto"
)

type MalResponse struct {
	Data   []MalItem  `json:"data"`
	Paging *MalPaging `json:"paging"`
}

type MalPaging struct {
	Next     *string `json:"next"`
	Previous *string `json:"previous"`
}

type MalItem struct {
	Node       MalNode        `json:"node"`
	ListStatus *MalListStatus `json:"list_status"`
}

type MalNode struct {
	ID                     *int64         `json:"id"`
	Title                  *string        `json:"title"`
	AlternativeTitles      *MalAltTitles  `json:"alternative_titles"`
	MainPicture            *MalPicture    `json:"main_picture"`
	Genres                 []MalGenre     `json:"genres"`
	Synopsis               *string        `json:"synopsis"`
	Mean                   *float32       `json:"mean"`
	Popularity             *int           `json:"popularity"`
	NumEpisodes            *int           `json:"num_episodes"`
	AverageEpisodeDuration *int           `json:"average_episode_duration"`
	NumChapters            *int           `json:"num_chapters"`
	NumVolumes             *int           `json:"num_volumes"`
	Nsfw                   *string        `json:"nsfw"`
	MyListStatus           *MalListStatus `json:"my_list_status"`
	MediaType              *string        `json:"media_type"`
	Status                 *string        `json:"status"`
}

type MalAltTitles struct {
	En *string `json:"en"`
	Ja *string `json:"ja"`
}

type MalPicture struct {
	Medium *string `json:"medium"`
	Large  *string `json:"large"`
}

type MalGenre struct {
	ID   *int64  `json:"id"`
	Name *string `json:"name"`
}

type MalListStatus struct {
	Status             *string  `json:"status"`
	Score              *float32 `json:"score"`
	NumEpisodesWatched *float64 `json:"num_episodes_watched"`
	NumChaptersRead    *int     `json:"num_chapters_read"`
	NumVolumesRead     *int     `json:"num_volumes_read"`
	IsRewatching       *bool    `json:"is_rewatching"`
	IsRereading        *bool    `json:"is_rereading"`
	NumTimesRewatched  *int     `json:"num_times_rewatched"`
	NumTimesReread     *int     `json:"num_times_reread"`
	Tags               []string `json:"tags"`
	Comments           *string  `json:"comments"`
	UpdatedAt          *string  `json:"updated_at"`
}

type MalStatus string

const (
	MalStatusWatching    MalStatus = "watching"
	MalStatusReading     MalStatus = "reading"
	MalStatusPlanToWatch MalStatus = "plan_to_watch"
	MalStatusPlanToRead  MalStatus = "plan_to_read"
	MalStatusOnHold      MalStatus = "on_hold"
	MalStatusCompleted   MalStatus = "completed"
	MalStatusDropped     MalStatus = "dropped"
)

func ParseMalStatus(status string, isManga bool) (MalStatus, error) {
	switch strings.ToLower(status) {
	case "current", "watching", "reading", "repeating", "rewatching", "rereading":
		if isManga {
			return MalStatusReading, nil
		}
		return MalStatusWatching, nil
	case "planning", "plan_to_watch", "plan_to_read":
		if isManga {
			return MalStatusPlanToRead, nil
		}
		return MalStatusPlanToWatch, nil
	case "paused", "on_hold":
		return MalStatusOnHold, nil
	case "completed":
		return MalStatusCompleted, nil
	case "dropped":
		return MalStatusDropped, nil
	default:
		return "", errors.New("unknown mal status: " + status)
	}
}

func parseMalListStatus(status *string, isRepeating bool) dto.ListStatus {
	if isRepeating {
		return dto.ListStatusRepeating
	}
	if status == nil {
		return dto.ListStatusCurrent
	}
	switch strings.ToLower(*status) {
	case "watching", "reading":
		return dto.ListStatusCurrent
	case "plan_to_watch", "plan_to_read":
		return dto.ListStatusPlanning
	case "completed":
		return dto.ListStatusCompleted
	case "dropped":
		return dto.ListStatusDropped
	case "on_hold":
		return dto.ListStatusPaused
	default:
		return dto.ListStatusCurrent
	}
}

func parseMalMediaFormat(mediaType *string) dto.MediaFormat {
	if mediaType == nil {
		return dto.MediaFormatUnknown
	}
	switch strings.ToLower(*mediaType) {
	case "tv":
		return dto.MediaFormatTv
	case "ova":
		return dto.MediaFormatOva
	case "movie":
		return dto.MediaFormatMovie
	case "special":
		return dto.MediaFormatSpecial
	case "ona":
		return dto.MediaFormatOna
	case "music":
		return dto.MediaFormatMusic
	case "manga":
		return dto.MediaFormatManga
	case "novel":
		return dto.MediaFormatNovel
	case "one_shot":
		return dto.MediaFormatOneShot
	case "doujinshi":
		return dto.MediaFormatDoujinshi
	case "manhwa":
		return dto.MediaFormatManhwa
	case "manhua":
		return dto.MediaFormatManhua
	case "oel":
		return dto.MediaFormatOel
	default:
		return dto.MediaFormatUnknown
	}
}

func parseMalReleaseStatus(status *string) dto.ReleaseStatus {
	if status == nil {
		return dto.ReleaseStatusUnknown
	}
	switch strings.ToLower(*status) {
	case "currently_airing", "currently_publishing":
		return dto.ReleaseStatusReleasing
	case "finished_airing", "finished":
		return dto.ReleaseStatusFinished
	case "not_yet_aired", "not_yet_published":
		return dto.ReleaseStatusNotYetReleased
	default:
		return dto.ReleaseStatusUnknown
	}
}

func parseMalNsfw(nsfw *string) dto.NsfwLevel {
	if nsfw == nil {
		return dto.NsfwLevelSafe
	}
	switch strings.ToLower(*nsfw) {
	case "white":
		return dto.NsfwLevelSafe
	case "gray":
		return dto.NsfwLevelGray
	case "black":
		return dto.NsfwLevelNsfw
	default:
		return dto.NsfwLevelSafe
	}
}

func (l *MalListStatus) ToListEntry() dto.ListEntry {
	if l == nil {
		return dto.ListEntry{
			Status: dto.ListStatusCurrent,
			Tags:   []string{},
		}
	}

	isRepeating := (l.IsRewatching != nil && *l.IsRewatching) || (l.IsRereading != nil && *l.IsRereading)
	status := parseMalListStatus(l.Status, isRepeating)

	var repeatCount *int
	if l.NumTimesRewatched != nil {
		repeatCount = l.NumTimesRewatched
	} else if l.NumTimesReread != nil {
		repeatCount = l.NumTimesReread
	}

	var progress *int
	if l.NumEpisodesWatched != nil {
		p := int(*l.NumEpisodesWatched)
		progress = &p
	} else if l.NumChaptersRead != nil {
		progress = l.NumChaptersRead
	}

	tags := l.Tags
	if tags == nil {
		tags = []string{}
	}

	return dto.ListEntry{
		Status:          status,
		Score:           l.Score,
		Progress:        progress,
		ProgressVolumes: l.NumVolumesRead,
		IsRepeating:     isRepeating,
		RepeatCount:     repeatCount,
		Tags:            tags,
		Notes:           l.Comments,
		UpdatedAt:       l.UpdatedAt,
	}
}

func getMalMediaType(format dto.MediaFormat, node *MalNode) dto.MediaType {
	switch format {
	case dto.MediaFormatManga,
		dto.MediaFormatNovel,
		dto.MediaFormatOneShot,
		dto.MediaFormatDoujinshi,
		dto.MediaFormatManhwa,
		dto.MediaFormatManhua,
		dto.MediaFormatOel:
		return dto.MediaTypeManga
	default:
		if node.NumChapters != nil || node.NumVolumes != nil {
			return dto.MediaTypeManga
		}
		return dto.MediaTypeAnime
	}
}

func (item *MalItem) ToMediaEntry() (dto.MediaEntry, error) {
	node := item.Node
	if node.ID == nil {
		return dto.MediaEntry{}, errors.New("missing node id")
	}
	if node.Title == nil {
		return dto.MediaEntry{}, errors.New("missing node title")
	}

	titleStr := *node.Title
	providerID := strconv.FormatInt(*node.ID, 10)

	listStatus := item.ListStatus
	if listStatus == nil {
		listStatus = node.MyListStatus
	}
	listEntry := listStatus.ToListEntry()

	var english, native *string
	if node.AlternativeTitles != nil {
		english = node.AlternativeTitles.En
		native = node.AlternativeTitles.Ja
	}

	var medium, large, extraLarge *string
	if node.MainPicture != nil {
		medium = node.MainPicture.Medium
		large = node.MainPicture.Large
	}
	if large != nil {
		extraLarge = large
	} else {
		extraLarge = medium
	}

	var genres []string
	if node.Genres != nil {
		genres = make([]string, 0, len(node.Genres))
		for _, g := range node.Genres {
			if g.Name != nil && *g.Name != "" {
				genres = append(genres, *g.Name)
			}
		}
	} else {
		genres = []string{}
	}

	format := parseMalMediaFormat(node.MediaType)
	mediaType := getMalMediaType(format, &node)

	media := dto.Media{
		ID:            uuid.Nil(),
		ProviderID:    providerID,
		Provider:      dto.MediaProviderMAL,
		MediaType:     mediaType,
		Format:        format,
		ReleaseStatus: parseMalReleaseStatus(node.Status),
		Title: dto.MediaTitle{
			Romanized:     &titleStr,
			English:       english,
			Native:        native,
			UserPreferred: &titleStr,
		},
		Cover: dto.CoverImage{
			ExtraLarge: extraLarge,
			Large:      large,
			Medium:     medium,
			Color:      nil,
		},
		Synopsis:   node.Synopsis,
		MeanScore:  node.Mean,
		Popularity: node.Popularity,
		Episodes:   node.NumEpisodes,
		Duration:   node.AverageEpisodeDuration,
		Chapters:   node.NumChapters,
		Volumes:    node.NumVolumes,
		Genres:     genres,
		Nsfw:       parseMalNsfw(node.Nsfw),
	}

	return dto.MediaEntry{
		Media:     media,
		ListEntry: listEntry,
	}, nil
}

func (res *MalResponse) ToPaginatedResponse() dto.PaginatedResponse {
	entries := make([]dto.MediaEntry, 0, len(res.Data))
	for i := range res.Data {
		entry, err := res.Data[i].ToMediaEntry()
		if err == nil {
			entries = append(entries, entry)
		}
	}

	var nextCursor, prevCursor *string
	hasNext := false
	if res.Paging != nil {
		nextCursor = res.Paging.Next
		prevCursor = res.Paging.Previous
		hasNext = nextCursor != nil && *nextCursor != ""
	}

	return dto.PaginatedResponse{
		Data: entries,
		Paging: dto.Paging{
			NextCursor: nextCursor,
			PrevCursor: prevCursor,
			HasNext:    hasNext,
		},
	}
}
