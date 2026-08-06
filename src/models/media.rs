use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
use strum::{AsRefStr, Display, EnumString};
use uuid::Uuid;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Enums
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    Type,
    Display,
    EnumString,
    AsRefStr,
)]
#[sqlx(type_name = "TEXT", rename_all = "UPPERCASE")]
pub enum MediaProvider {
    #[default]
    MAL,
    ANILIST,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum MediaType {
    #[default]
    Anime,
    Manga,
}

/// What format the media was released as.
///
/// Unified superset of MAL media_type + AniList MediaFormat.
/// MAL anime: unknown, tv, ova, movie, special, ona, music
/// MAL manga: unknown, manga, novel, one_shot, doujinshi, manhwa, manhua, oel
/// AniList:   TV, TV_SHORT, MOVIE, SPECIAL, OVA, ONA, MUSIC, MANGA, NOVEL, ONE_SHOT
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum MediaFormat {
    #[default]
    Unknown,
    Tv,
    TvShort,
    Movie,
    Special,
    Ova,
    Ona,
    Music,
    Manga,
    Novel,
    OneShot,
    Doujinshi,
    Manhwa,
    Manhua,
    Oel,
}

/// Airing / publishing status of the media itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ReleaseStatus {
    #[default]
    Unknown,
    Releasing,
    Finished,
    NotYetReleased,
    Cancelled,
    Hiatus,
}

/// User's personal watching/reading status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ListStatus {
    #[default]
    Current,
    Planning,
    Completed,
    Dropped,
    Paused,
    Repeating,
}

/// NSFW classification.
/// MAL distinguishes white/gray/black. AniList only has isAdult bool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum NsfwLevel {
    #[default]
    Safe,
    Gray,
    Nsfw,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Value objects
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MediaTitle {
    /// AniList: romaji. MAL: title (the main title IS romanized).
    pub romanized: Option<String>,
    /// AniList: english. MAL: alternative_titles.en.
    pub english: Option<String>,
    /// AniList: native. MAL: alternative_titles.ja.
    pub native: Option<String>,
    /// AniList: userPreferred. MAL: same as romanized.
    pub user_preferred: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CoverImage {
    /// AniList only. Falls back to large if unavailable.
    pub extra_large: Option<String>,
    /// Both APIs.
    pub large: Option<String>,
    /// Both APIs.
    pub medium: Option<String>,
    /// AniList only. Average hex color of the cover.
    pub color: Option<String>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Core models
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The media itself — anime or manga metadata.
/// No user state. No list status. Just what the thing IS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Media {
    pub id: Uuid,

    /// Provider-specific ID (MAL int or AniList int, as string).
    pub provider_id: String,
    pub provider: MediaProvider,

    pub media_type: MediaType,
    pub format: MediaFormat,
    pub release_status: ReleaseStatus,

    pub title: MediaTitle,
    pub cover: CoverImage,
    pub synopsis: Option<String>,

    /// Community mean score. MAL: 0.0–10.0. AniList: 0–100 (normalize to 0–10).
    pub mean_score: Option<f32>,
    pub popularity: Option<i32>,

    // ── Type-specific counts ──
    /// Anime only. Total episode count (0 or null if unknown).
    pub episodes: Option<i32>,
    /// Anime only. Episode length in seconds.
    pub duration: Option<i32>,
    /// Manga only.
    pub chapters: Option<i32>,
    /// Manga only.
    pub volumes: Option<i32>,

    pub genres: Vec<String>,
    pub nsfw: NsfwLevel,
}

/// What the user thinks of a specific media — their list entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListEntry {
    pub status: ListStatus,

    /// User's score. Normalized to 0.0–10.0.
    pub score: Option<f32>,

    /// Episodes watched (anime) or chapters read (manga).
    pub progress: Option<i32>,
    /// Volumes read (manga only).
    pub progress_volumes: Option<i32>,

    pub is_repeating: bool,
    pub repeat_count: Option<i32>,

    pub tags: Vec<String>,
    /// AniList: notes. MAL: comments.
    pub notes: Option<String>,

    pub updated_at: Option<String>,
}

/// The "edge" that joins a media with a user's list entry.
/// This is what the API returns: one of these per item in the list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MediaEntry {
    pub media: Media,
    pub list_entry: ListEntry,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Pagination
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Paging {
    /// MAL provides cursor URLs. AniList uses page numbers.
    /// Store the raw cursor/URL if the provider gives one.
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
    pub has_next: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PaginatedResponse {
    pub data: Vec<MediaEntry>,
    pub paging: Paging,
}
