package dto

import "uuid"

// MediaFormat defines what format the media was released as.
// Unified superset of MAL media_type + AniList MediaFormat.
// MAL anime: unknown, tv, ova, movie, special, ona, music
// MAL manga: unknown, manga, novel, one_shot, doujinshi, manhwa, manhua, oel
// AniList:   TV, TV_SHORT, MOVIE, SPECIAL, OVA, ONA, MUSIC, MANGA, NOVEL, ONE_SHOT
type MediaFormat string

const (
	MediaFormatUnknown   MediaFormat = "Unknown"
	MediaFormatTv        MediaFormat = "Tv"
	MediaFormatTvShort   MediaFormat = "TvShort"
	MediaFormatMovie     MediaFormat = "Movie"
	MediaFormatSpecial   MediaFormat = "Special"
	MediaFormatOva       MediaFormat = "Ova"
	MediaFormatOna       MediaFormat = "Ona"
	MediaFormatMusic     MediaFormat = "Music"
	MediaFormatManga     MediaFormat = "Manga"
	MediaFormatNovel     MediaFormat = "Novel"
	MediaFormatOneShot   MediaFormat = "OneShot"
	MediaFormatDoujinshi MediaFormat = "Doujinshi"
	MediaFormatManhwa    MediaFormat = "Manhwa"
	MediaFormatManhua    MediaFormat = "Manhua"
	MediaFormatOel       MediaFormat = "Oel"
)

// ReleaseStatus defines the airing / publishing status of the media itself.
type ReleaseStatus string

const (
	ReleaseStatusUnknown        ReleaseStatus = "Unknown"
	ReleaseStatusReleasing      ReleaseStatus = "Releasing"
	ReleaseStatusFinished       ReleaseStatus = "Finished"
	ReleaseStatusNotYetReleased ReleaseStatus = "NotYetReleased"
	ReleaseStatusCancelled      ReleaseStatus = "Cancelled"
	ReleaseStatusHiatus         ReleaseStatus = "Hiatus"
)

// ListStatus defines user's personal watching/reading status.
type ListStatus string

const (
	ListStatusCurrent   ListStatus = "Current"
	ListStatusPlanning  ListStatus = "Planning"
	ListStatusCompleted ListStatus = "Completed"
	ListStatusDropped   ListStatus = "Dropped"
	ListStatusPaused    ListStatus = "Paused"
	ListStatusRepeating ListStatus = "Repeating"
)

// NsfwLevel defines NSFW classification.
// MAL distinguishes white/gray/black. AniList only has isAdult bool.
type NsfwLevel string

const (
	NsfwLevelSafe NsfwLevel = "Safe"
	NsfwLevelGray NsfwLevel = "Gray"
	NsfwLevelNsfw NsfwLevel = "Nsfw"
)

type MediaProvider string

const (
	MediaProviderMAL     MediaProvider = "MAL"
	MediaProviderAnilist MediaProvider = "ANILIST"
)

type MediaType string

const (
	MediaTypeAnime MediaType = "Anime"
	MediaTypeManga MediaType = "Manga"
	MediaTypeNovel MediaType = "Novel"
)

type MediaTitle struct {
	// AniList: romaji. MAL: title (the main title IS romanized).
	Romanized *string `json:"romanized"`
	// AniList: english. MAL: alternative_titles.en.
	English *string `json:"english"`
	// AniList: native. MAL: alternative_titles.ja.
	Native *string `json:"native"`
	// AniList: userPreferred. MAL: same as romanized.
	UserPreferred *string `json:"user_preferred"`
}

type CoverImage struct {
	// AniList only. Falls back to large if unavailable.
	ExtraLarge *string `json:"extra_large"`
	// Both APIs.
	Large *string `json:"large"`
	// Both APIs.
	Medium *string `json:"medium"`
	// AniList only. Average hex color of the cover.
	Color *string `json:"color"`
}

// Media represents the media itself — anime or manga metadata.
// No user state. No list status. Just what the thing IS.
type Media struct {
	ID uuid.UUID `json:"id"`

	// Provider-specific ID (MAL int or AniList int, as string).
	ProviderID string        `json:"provider_id"`
	Provider   MediaProvider `json:"provider"`

	MediaType     MediaType     `json:"media_type"`
	Format        MediaFormat   `json:"format"`
	ReleaseStatus ReleaseStatus `json:"release_status"`

	Title    MediaTitle `json:"title"`
	Cover    CoverImage `json:"cover"`
	Synopsis *string    `json:"synopsis"`

	// Community mean score. MAL: 0.0–10.0. AniList: 0–100 (normalize to 0–10).
	MeanScore  *float32 `json:"mean_score"`
	Popularity *int     `json:"popularity"`

	// Type-specific counts
	// Anime only. Total episode count (0 or null if unknown).
	Episodes *int `json:"episodes"`
	// Anime only. Episode length in seconds.
	Duration *int `json:"duration"`
	// Manga only.
	Chapters *int `json:"chapters"`
	// Manga only.
	Volumes *int `json:"volumes"`

	Genres []string  `json:"genres"`
	Nsfw   NsfwLevel `json:"nsfw"`
}

// ListEntry represents what the user thinks of a specific media — their list entry.
type ListEntry struct {
	Status ListStatus `json:"status"`

	// User's score. Normalized to 0.0–10.0.
	Score *float32 `json:"score"`

	// Episodes watched (anime) or chapters read (manga).
	Progress *int `json:"progress"`
	// Volumes read (manga only).
	ProgressVolumes *int `json:"progress_volumes"`

	IsRepeating bool `json:"is_repeating"`
	RepeatCount *int `json:"repeat_count"`

	Tags []string `json:"tags"`
	// AniList: notes. MAL: comments.
	Notes *string `json:"notes"`

	UpdatedAt *string `json:"updated_at"`
}

// MediaEntry is the "edge" that joins a media with a user's list entry.
// This is what the API returns: one of these per item in the list.
type MediaEntry struct {
	Media     Media     `json:"media"`
	ListEntry ListEntry `json:"list_entry"`
}

type Paging struct {
	// MAL provides cursor URLs. AniList uses page numbers.
	// Store the raw cursor/URL if the provider gives one.
	NextCursor *string `json:"next_cursor"`
	PrevCursor *string `json:"prev_cursor"`
	HasNext    bool    `json:"has_next"`
}

type PaginatedResponse struct {
	Data   []MediaEntry `json:"data"`
	Paging Paging       `json:"paging"`
}
