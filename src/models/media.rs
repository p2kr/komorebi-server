use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MediaProvider {
    MAL,
    ANILIST,
    SANDBOX,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MediaType {
    ANIME,
    MANGA,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Contains title variants harmonized across MAL and AniList.
pub struct MediaTitle {
    romanized: String,
    english: String,
    native: String,
    user_preferred: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Contains image URL variants and optional accent color code.
pub struct MediaCoverImage {
    extra_large: String,
    large: String,
    medium: String,
    color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Represents user library status details.
pub struct MediaListStatus {
    status: Option<String>,
    score: Option<f32>,
    progress: Option<f32>,
    progress_volumes: Option<i32>,
    is_rewatching: bool,
    repeat_count: Option<i32>,
    tags: Vec<String>,
    comments: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Represents a harmonized anime or manga entry.
pub struct MediaItem {
    id: Uuid,
    provider_id: Option<String>,
    provider: MediaProvider,
    title: MediaTitle,
    cover_img: MediaCoverImage,
    synopsis: Option<String>,
    status: MediaListStatus,
    media_type: MediaType,
    mean_score: Option<f32>,
    rank: Option<i32>,
    popularity: Option<i32>,
    episodes: Option<i32>,
    chapters: Option<i32>,
    volumes: Option<i32>,
    seasons: Option<i32>,
    current_season: Option<i32>,
    duration: Option<i32>,
    genres: Vec<String>,
    is_nsfw: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Describes pagination metadata.
pub struct PagingInfo {
    prev: Option<String>,
    next: Option<String>,
    has_next: bool,
    page: i32,
    per_page: i32,
    max_pages: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Pagination wrapper for media item lists.
pub struct PaginatedResponse {
    data: Vec<MediaItem>,
    paging_info: PagingInfo,
}
