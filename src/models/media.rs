use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::user::User;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum MediaProvider {
    MAL,
    ANILIST,
    #[default]
    SANDBOX,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum MediaType {
    #[default]
    ANIME,
    MANGA,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Contains title variants harmonized across MAL and AniList.
pub struct MediaTitle {
    pub romanized: String,
    pub english: String,
    pub native: String,
    pub user_preferred: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Contains image URL variants and optional accent color code.
pub struct MediaCoverImage {
    pub extra_large: String,
    pub large: String,
    pub medium: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Represents user library status details.
pub struct MediaListStatus {
    pub status: Option<String>,
    pub score: Option<f32>,
    pub progress: Option<f32>,
    pub progress_volumes: Option<i32>,
    pub is_rewatching: bool,
    pub repeat_count: Option<i32>,
    pub tags: Vec<String>,
    pub comments: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Represents a harmonized anime or manga entry.
pub struct MediaItem {
    pub id: Uuid,
    pub provider_id: Option<String>,
    pub provider: MediaProvider,
    pub user: User,
    pub title: MediaTitle,
    pub cover_img: MediaCoverImage,
    pub synopsis: Option<String>,
    pub status: MediaListStatus,
    pub media_type: MediaType,
    pub mean_score: Option<f32>,
    pub rank: Option<i32>,
    pub popularity: Option<i32>,
    pub episodes: Option<i32>,
    pub chapters: Option<i32>,
    pub volumes: Option<i32>,
    pub seasons: Option<i32>,
    pub current_season: Option<i32>,
    pub duration: Option<i32>,
    pub genres: Vec<String>,
    pub is_nsfw: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Describes pagination metadata.
pub struct PagingInfo {
    pub prev: Option<String>,
    pub next: Option<String>,
    pub has_next: bool,
    pub page: i32,
    pub per_page: i32,
    pub max_pages: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Pagination wrapper for media item lists.
pub struct PaginatedResponse {
    pub data: Vec<MediaItem>,
    pub paging_info: PagingInfo,
}
