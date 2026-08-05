use serde::Deserialize;
use uuid::Uuid;

use crate::models::media::{
    CoverImage, ListEntry, ListStatus, Media, MediaEntry, MediaFormat, MediaProvider, MediaTitle,
    MediaType, NsfwLevel, PaginatedResponse, Paging, ReleaseStatus,
};

/// AniList GraphQL API response wrapper.
#[derive(Debug, Deserialize)]
pub struct AniListResponse {
    pub data: Option<AniListData>,
    pub errors: Option<Vec<AniListGraphqlError>>,
}

#[derive(Debug, Deserialize)]
pub struct AniListGraphqlError {
    pub message: String,
    pub status: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AniListData {
    #[serde(rename = "Page")]
    pub page: Option<AniListPage>,
}

#[derive(Debug, Deserialize)]
pub struct AniListPage {
    #[serde(rename = "pageInfo")]
    pub page_info: Option<AniListPageInfo>,
    #[serde(rename = "mediaList")]
    pub media_list: Option<Vec<AniListMediaListEntry>>,
}

#[derive(Debug, Deserialize)]
pub struct AniListPageInfo {
    pub total: Option<i32>,
    #[serde(rename = "perPage")]
    pub per_page: Option<i32>,
    #[serde(rename = "currentPage")]
    pub current_page: Option<i32>,
    #[serde(rename = "lastPage")]
    pub last_page: Option<i32>,
    #[serde(rename = "hasNextPage")]
    pub has_next_page: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AniListMediaListEntry {
    pub id: Option<i64>,
    pub status: Option<String>,
    pub score: Option<f64>,
    pub progress: Option<i32>,
    #[serde(rename = "progressVolumes")]
    pub progress_volumes: Option<i32>,
    pub repeat: Option<i32>,
    pub notes: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<i64>,
    pub media: Option<AniListMedia>,
}

#[derive(Debug, Deserialize)]
pub struct AniListMedia {
    pub id: Option<i64>,
    #[serde(rename = "idMal")]
    pub id_mal: Option<i64>,
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    pub format: Option<String>,
    pub status: Option<String>,
    pub title: Option<AniListTitle>,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<AniListCoverImage>,
    pub description: Option<String>,
    #[serde(rename = "meanScore")]
    pub mean_score: Option<f64>,
    pub popularity: Option<i32>,
    pub episodes: Option<i32>,
    pub duration: Option<i32>,
    pub chapters: Option<i32>,
    pub volumes: Option<i32>,
    pub genres: Option<Vec<String>>,
    #[serde(rename = "isAdult")]
    pub is_adult: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AniListTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
    #[serde(rename = "userPreferred")]
    pub user_preferred: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AniListCoverImage {
    #[serde(rename = "extraLarge")]
    pub extra_large: Option<String>,
    pub large: Option<String>,
    pub medium: Option<String>,
    pub color: Option<String>,
}

fn parse_anilist_list_status(status_str: Option<&str>) -> ListStatus {
    match status_str {
        Some("CURRENT") => ListStatus::Current,
        Some("PLANNING") => ListStatus::Planning,
        Some("COMPLETED") => ListStatus::Completed,
        Some("DROPPED") => ListStatus::Dropped,
        Some("PAUSED") => ListStatus::Paused,
        Some("REPEATING") => ListStatus::Repeating,
        _ => ListStatus::Current,
    }
}

fn parse_anilist_media_type(type_str: Option<&str>) -> MediaType {
    match type_str {
        Some("ANIME") => MediaType::Anime,
        Some("MANGA") => MediaType::Manga,
        _ => MediaType::Anime,
    }
}

fn parse_anilist_media_format(format_str: Option<&str>) -> MediaFormat {
    match format_str {
        Some("TV") => MediaFormat::Tv,
        Some("TV_SHORT") => MediaFormat::TvShort,
        Some("MOVIE") => MediaFormat::Movie,
        Some("SPECIAL") => MediaFormat::Special,
        Some("OVA") => MediaFormat::Ova,
        Some("ONA") => MediaFormat::Ona,
        Some("MUSIC") => MediaFormat::Music,
        Some("MANGA") => MediaFormat::Manga,
        Some("NOVEL") => MediaFormat::Novel,
        Some("ONE_SHOT") => MediaFormat::OneShot,
        _ => MediaFormat::Unknown,
    }
}

fn parse_anilist_release_status(status_str: Option<&str>) -> ReleaseStatus {
    match status_str {
        Some("RELEASING") => ReleaseStatus::Releasing,
        Some("FINISHED") => ReleaseStatus::Finished,
        Some("NOT_YET_RELEASED") => ReleaseStatus::NotYetReleased,
        Some("CANCELLED") => ReleaseStatus::Cancelled,
        Some("HIATUS") => ReleaseStatus::Hiatus,
        _ => ReleaseStatus::Unknown,
    }
}

impl From<AniListMediaListEntry> for ListEntry {
    fn from(entry: AniListMediaListEntry) -> Self {
        let status = parse_anilist_list_status(entry.status.as_deref());
        let is_repeating = status == ListStatus::Repeating || entry.repeat.unwrap_or(0) > 0;

        ListEntry {
            status,
            score: entry.score.map(|s| s as f32),
            progress: entry.progress,
            progress_volumes: entry.progress_volumes,
            is_repeating,
            repeat_count: entry.repeat,
            tags: Vec::new(),
            notes: entry.notes,
            updated_at: entry.updated_at.map(|t| t.to_string()),
        }
    }
}

impl TryFrom<AniListMediaListEntry> for MediaEntry {
    type Error = ();

    fn try_from(mut entry: AniListMediaListEntry) -> Result<Self, Self::Error> {
        let media_node = entry.media.take().ok_or(())?;
        let provider_id = media_node.id.ok_or(())?.to_string();

        let title = match media_node.title {
            Some(t) => MediaTitle {
                romanized: t.romaji,
                english: t.english,
                native: t.native,
                user_preferred: t.user_preferred,
            },
            None => MediaTitle::default(),
        };

        let cover = match media_node.cover_image {
            Some(c) => CoverImage {
                extra_large: c.extra_large,
                large: c.large,
                medium: c.medium,
                color: c.color,
            },
            None => CoverImage::default(),
        };

        let is_nsfw = media_node.is_adult.unwrap_or(false);

        let media = Media {
            id: Uuid::now_v7(),
            provider_id,
            provider: MediaProvider::ANILIST,
            media_type: parse_anilist_media_type(media_node.media_type.as_deref()),
            format: parse_anilist_media_format(media_node.format.as_deref()),
            release_status: parse_anilist_release_status(media_node.status.as_deref()),
            title,
            cover,
            synopsis: media_node.description,
            // AniList meanScore is 0-100, normalize to 0.0-10.0 scale
            mean_score: media_node.mean_score.map(|s| (s / 10.0) as f32),
            popularity: media_node.popularity,
            episodes: media_node.episodes,
            duration: media_node.duration.map(|mins| mins * 60), // convert minutes to seconds
            chapters: media_node.chapters,
            volumes: media_node.volumes,
            genres: media_node.genres.unwrap_or_default(),
            nsfw: if is_nsfw {
                NsfwLevel::Nsfw
            } else {
                NsfwLevel::Safe
            },
        };

        let list_entry = ListEntry::from(entry);

        Ok(MediaEntry { media, list_entry })
    }
}

impl From<AniListResponse> for PaginatedResponse {
    fn from(res: AniListResponse) -> Self {
        let (data_entries, page_info) = match res.data.and_then(|d| d.page) {
            Some(page) => (page.media_list.unwrap_or_default(), page.page_info),
            None => (Vec::new(), None),
        };

        let entries: Vec<MediaEntry> = data_entries
            .into_iter()
            .filter_map(|item| item.try_into().ok())
            .collect();

        let (has_next, next_cursor, prev_cursor) = match page_info {
            Some(info) => {
                let has_next = info.has_next_page.unwrap_or(false);
                let next = if has_next {
                    info.current_page.map(|p| (p + 1).to_string())
                } else {
                    None
                };
                let prev = info.current_page.and_then(|p| {
                    if p > 1 {
                        Some((p - 1).to_string())
                    } else {
                        None
                    }
                });
                (has_next, next, prev)
            }
            None => (false, None, None),
        };

        PaginatedResponse {
            data: entries,
            paging: Paging {
                next_cursor,
                prev_cursor,
                has_next,
            },
        }
    }
}
