package anilist

import (
	"errors"
	"strconv"
	"strings"
	"uuid"

	"komorebi-server/src/dto"
)

type AniListResponse struct {
	Data   *AniListData          `json:"data"`
	Errors []AniListGraphqlError `json:"errors"`
}

type AniListGraphqlError struct {
	Message string `json:"message"`
	Status  *int   `json:"status"`
}

type AniListData struct {
	Page *AniListPage `json:"Page"`
}

type AniListPage struct {
	PageInfo  *AniListPageInfo         `json:"pageInfo"`
	MediaList []AniListMediaListEntry `json:"mediaList"`
}

type AniListPageInfo struct {
	Total       *int  `json:"total"`
	PerPage     *int  `json:"perPage"`
	CurrentPage *int  `json:"currentPage"`
	LastPage    *int  `json:"lastPage"`
	HasNextPage *bool `json:"hasNextPage"`
}

type AniListMediaListEntry struct {
	ID              *int64        `json:"id"`
	Status          *string       `json:"status"`
	Score           *float32      `json:"score"`
	Progress        *int          `json:"progress"`
	ProgressVolumes *int          `json:"progressVolumes"`
	Repeat          *int          `json:"repeat"`
	Notes           *string       `json:"notes"`
	UpdatedAt       *int64        `json:"updatedAt"`
	Media           *AniListMedia `json:"media"`
}

type AniListMedia struct {
	ID          *int64             `json:"id"`
	IDMal       *int64             `json:"idMal"`
	MediaType   *string            `json:"type"`
	Format      *string            `json:"format"`
	Status      *string            `json:"status"`
	Title       *AniListTitle      `json:"title"`
	CoverImage  *AniListCoverImage `json:"coverImage"`
	Description *string            `json:"description"`
	MeanScore   *float32           `json:"meanScore"`
	Popularity  *int               `json:"popularity"`
	Episodes    *int               `json:"episodes"`
	Duration    *int               `json:"duration"`
	Chapters    *int               `json:"chapters"`
	Volumes     *int               `json:"volumes"`
	Genres      []string           `json:"genres"`
	IsAdult     *bool              `json:"isAdult"`
}

type AniListTitle struct {
	Romaji        *string `json:"romaji"`
	English       *string `json:"english"`
	Native        *string `json:"native"`
	UserPreferred *string `json:"userPreferred"`
}

type AniListCoverImage struct {
	ExtraLarge *string `json:"extraLarge"`
	Large      *string `json:"large"`
	Medium     *string `json:"medium"`
	Color      *string `json:"color"`
}

func parseAniListListStatus(statusStr *string) dto.ListStatus {
	if statusStr == nil {
		return dto.ListStatusCurrent
	}
	switch strings.ToUpper(*statusStr) {
	case "CURRENT":
		return dto.ListStatusCurrent
	case "PLANNING":
		return dto.ListStatusPlanning
	case "COMPLETED":
		return dto.ListStatusCompleted
	case "DROPPED":
		return dto.ListStatusDropped
	case "PAUSED":
		return dto.ListStatusPaused
	case "REPEATING":
		return dto.ListStatusRepeating
	default:
		return dto.ListStatusCurrent
	}
}

func parseAniListMediaType(typeStr *string) dto.MediaType {
	if typeStr == nil {
		return dto.MediaTypeAnime
	}
	switch strings.ToUpper(*typeStr) {
	case "ANIME":
		return dto.MediaTypeAnime
	case "MANGA":
		return dto.MediaTypeManga
	default:
		return dto.MediaTypeAnime
	}
}

func parseAniListMediaFormat(formatStr *string) dto.MediaFormat {
	if formatStr == nil {
		return dto.MediaFormatUnknown
	}
	switch strings.ToUpper(*formatStr) {
	case "TV":
		return dto.MediaFormatTv
	case "TV_SHORT":
		return dto.MediaFormatTvShort
	case "MOVIE":
		return dto.MediaFormatMovie
	case "SPECIAL":
		return dto.MediaFormatSpecial
	case "OVA":
		return dto.MediaFormatOva
	case "ONA":
		return dto.MediaFormatOna
	case "MUSIC":
		return dto.MediaFormatMusic
	case "MANGA":
		return dto.MediaFormatManga
	case "NOVEL":
		return dto.MediaFormatNovel
	case "ONE_SHOT":
		return dto.MediaFormatOneShot
	default:
		return dto.MediaFormatUnknown
	}
}

func parseAniListReleaseStatus(statusStr *string) dto.ReleaseStatus {
	if statusStr == nil {
		return dto.ReleaseStatusUnknown
	}
	switch strings.ToUpper(*statusStr) {
	case "RELEASING":
		return dto.ReleaseStatusReleasing
	case "FINISHED":
		return dto.ReleaseStatusFinished
	case "NOT_YET_RELEASED":
		return dto.ReleaseStatusNotYetReleased
	case "CANCELLED":
		return dto.ReleaseStatusCancelled
	case "HIATUS":
		return dto.ReleaseStatusHiatus
	default:
		return dto.ReleaseStatusUnknown
	}
}

func (entry *AniListMediaListEntry) ToListEntry() dto.ListEntry {
	status := parseAniListListStatus(entry.Status)
	isRepeating := status == dto.ListStatusRepeating || (entry.Repeat != nil && *entry.Repeat > 0)

	var updatedAtStr *string
	if entry.UpdatedAt != nil {
		s := strconv.FormatInt(*entry.UpdatedAt, 10)
		updatedAtStr = &s
	}

	return dto.ListEntry{
		Status:          status,
		Score:           entry.Score,
		Progress:        entry.Progress,
		ProgressVolumes: entry.ProgressVolumes,
		IsRepeating:     isRepeating,
		RepeatCount:     entry.Repeat,
		Tags:            []string{},
		Notes:           entry.Notes,
		UpdatedAt:       updatedAtStr,
	}
}

func (entry *AniListMediaListEntry) ToMediaEntry() (dto.MediaEntry, error) {
	if entry.Media == nil {
		return dto.MediaEntry{}, errors.New("missing media node")
	}
	mediaNode := entry.Media
	if mediaNode.ID == nil {
		return dto.MediaEntry{}, errors.New("missing media id")
	}

	providerID := strconv.FormatInt(*mediaNode.ID, 10)

	var title dto.MediaTitle
	if mediaNode.Title != nil {
		title = dto.MediaTitle{
			Romanized:     mediaNode.Title.Romaji,
			English:       mediaNode.Title.English,
			Native:        mediaNode.Title.Native,
			UserPreferred: mediaNode.Title.UserPreferred,
		}
	}

	var cover dto.CoverImage
	if mediaNode.CoverImage != nil {
		cover = dto.CoverImage{
			ExtraLarge: mediaNode.CoverImage.ExtraLarge,
			Large:      mediaNode.CoverImage.Large,
			Medium:     mediaNode.CoverImage.Medium,
			Color:      mediaNode.CoverImage.Color,
		}
	}

	var meanScore *float32
	if mediaNode.MeanScore != nil {
		normalized := *mediaNode.MeanScore / 10.0
		meanScore = &normalized
	}

	var duration *int
	if mediaNode.Duration != nil {
		secs := *mediaNode.Duration * 60
		duration = &secs
	}

	isNsfw := mediaNode.IsAdult != nil && *mediaNode.IsAdult
	nsfwLevel := dto.NsfwLevelSafe
	if isNsfw {
		nsfwLevel = dto.NsfwLevelNsfw
	}

	genres := mediaNode.Genres
	if genres == nil {
		genres = []string{}
	}

	media := dto.Media{
		ID:            uuid.Nil(),
		ProviderID:    providerID,
		Provider:      dto.MediaProviderAnilist,
		MediaType:     parseAniListMediaType(mediaNode.MediaType),
		Format:        parseAniListMediaFormat(mediaNode.Format),
		ReleaseStatus: parseAniListReleaseStatus(mediaNode.Status),
		Title:         title,
		Cover:         cover,
		Synopsis:      mediaNode.Description,
		MeanScore:     meanScore,
		Popularity:    mediaNode.Popularity,
		Episodes:      mediaNode.Episodes,
		Duration:      duration,
		Chapters:      mediaNode.Chapters,
		Volumes:       mediaNode.Volumes,
		Genres:        genres,
		Nsfw:          nsfwLevel,
	}

	listEntry := entry.ToListEntry()

	return dto.MediaEntry{
		Media:     media,
		ListEntry: listEntry,
	}, nil
}

func (res *AniListResponse) ToPaginatedResponse() dto.PaginatedResponse {
	var mediaList []AniListMediaListEntry
	var pageInfo *AniListPageInfo

	if res.Data != nil && res.Data.Page != nil {
		mediaList = res.Data.Page.MediaList
		pageInfo = res.Data.Page.PageInfo
	}

	entries := make([]dto.MediaEntry, 0, len(mediaList))
	for i := range mediaList {
		entry, err := mediaList[i].ToMediaEntry()
		if err == nil {
			entries = append(entries, entry)
		}
	}

	hasNext := false
	var nextCursor, prevCursor *string
	if pageInfo != nil {
		if pageInfo.HasNextPage != nil && *pageInfo.HasNextPage {
			hasNext = true
		}
		if hasNext && pageInfo.CurrentPage != nil {
			next := strconv.Itoa(*pageInfo.CurrentPage + 1)
			nextCursor = &next
		}
		if pageInfo.CurrentPage != nil && *pageInfo.CurrentPage > 1 {
			prev := strconv.Itoa(*pageInfo.CurrentPage - 1)
			prevCursor = &prev
		}
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
