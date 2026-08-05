use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::media::{
    CoverImage, ListEntry, ListStatus, Media, MediaEntry, MediaFormat, MediaProvider, MediaTitle,
    MediaType, NsfwLevel, PaginatedResponse, Paging, ReleaseStatus,
};

#[derive(Debug, Deserialize)]
pub struct MalResponse {
    pub data: Vec<MalItem>,
    pub paging: Option<MalPaging>,
}

#[derive(Debug, Deserialize)]
pub struct MalPaging {
    pub next: Option<String>,
    pub previous: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MalItem {
    pub node: MalNode,
    pub list_status: Option<MalListStatus>,
}

#[derive(Debug, Deserialize)]
pub struct MalNode {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub alternative_titles: Option<MalAltTitles>,
    pub main_picture: Option<MalPicture>,
    pub genres: Option<Vec<MalGenre>>,
    pub synopsis: Option<String>,
    pub mean: Option<f64>,
    pub popularity: Option<i64>,
    pub num_episodes: Option<i64>,
    pub average_episode_duration: Option<i64>,
    pub num_chapters: Option<i64>,
    pub num_volumes: Option<i64>,
    pub nsfw: Option<String>,
    pub my_list_status: Option<MalListStatus>,
    pub media_type: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MalAltTitles {
    pub en: Option<String>,
    pub ja: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MalPicture {
    pub medium: Option<String>,
    pub large: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MalGenre {
    pub id: Option<i64>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MalListStatus {
    pub status: Option<String>,
    pub score: Option<f64>,
    pub num_episodes_watched: Option<f64>,
    pub num_chapters_read: Option<i64>,
    pub num_volumes_read: Option<i64>,
    pub is_rewatching: Option<bool>,
    pub is_rereading: Option<bool>,
    pub num_times_rewatched: Option<i64>,
    pub num_times_reread: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub comments: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MalStatus {
    Watching,
    Reading,
    PlanToWatch,
    PlanToRead,
    OnHold,
    Completed,
    Dropped,
}

impl TryFrom<(&str, bool)> for MalStatus {
    type Error = ();

    fn try_from((status, is_manga): (&str, bool)) -> Result<Self, Self::Error> {
        match (status.to_lowercase().as_str(), is_manga) {
            (
                "current" | "watching" | "reading" | "repeating" | "rewatching" | "rereading",
                true,
            ) => Ok(Self::Reading),
            (
                "current" | "watching" | "reading" | "repeating" | "rewatching" | "rereading",
                false,
            ) => Ok(Self::Watching),
            ("planning" | "plan_to_watch" | "plan_to_read", true) => Ok(Self::PlanToRead),
            ("planning" | "plan_to_watch" | "plan_to_read", false) => Ok(Self::PlanToWatch),
            ("paused" | "on_hold", _) => Ok(Self::OnHold),
            ("completed", _) => Ok(Self::Completed),
            ("dropped", _) => Ok(Self::Dropped),
            _ => Err(()),
        }
    }
}

fn parse_mal_list_status(status_str: Option<&str>, is_repeating: bool) -> ListStatus {
    if is_repeating {
        return ListStatus::Repeating;
    }
    match status_str {
        Some("watching") | Some("reading") => ListStatus::Current,
        Some("plan_to_watch") | Some("plan_to_read") => ListStatus::Planning,
        Some("completed") => ListStatus::Completed,
        Some("dropped") => ListStatus::Dropped,
        Some("on_hold") => ListStatus::Paused,
        _ => ListStatus::Current,
    }
}

fn parse_mal_media_format(media_type: Option<&str>) -> MediaFormat {
    match media_type {
        Some("tv") => MediaFormat::Tv,
        Some("ova") => MediaFormat::Ova,
        Some("movie") => MediaFormat::Movie,
        Some("special") => MediaFormat::Special,
        Some("ona") => MediaFormat::Ona,
        Some("music") => MediaFormat::Music,
        Some("manga") => MediaFormat::Manga,
        Some("novel") => MediaFormat::Novel,
        Some("one_shot") => MediaFormat::OneShot,
        Some("doujinshi") => MediaFormat::Doujinshi,
        Some("manhwa") => MediaFormat::Manhwa,
        Some("manhua") => MediaFormat::Manhua,
        Some("oel") => MediaFormat::Oel,
        _ => MediaFormat::Unknown,
    }
}

fn parse_mal_release_status(status_str: Option<&str>) -> ReleaseStatus {
    match status_str {
        Some("currently_airing") | Some("currently_publishing") => ReleaseStatus::Releasing,
        Some("finished_airing") | Some("finished") => ReleaseStatus::Finished,
        Some("not_yet_aired") | Some("not_yet_published") => ReleaseStatus::NotYetReleased,
        _ => ReleaseStatus::Unknown,
    }
}

fn parse_mal_nsfw(nsfw_str: Option<&str>) -> NsfwLevel {
    match nsfw_str {
        Some("white") => NsfwLevel::Safe,
        Some("gray") => NsfwLevel::Gray,
        Some("black") => NsfwLevel::Nsfw,
        _ => NsfwLevel::Safe,
    }
}

impl From<MalListStatus> for ListStatus {
    fn from(l: MalListStatus) -> Self {
        let is_repeating = l.is_rewatching.unwrap_or(false) || l.is_rereading.unwrap_or(false);
        parse_mal_list_status(l.status.as_deref(), is_repeating)
    }
}

impl From<MalListStatus> for ListEntry {
    fn from(l: MalListStatus) -> Self {
        let is_repeating = l.is_rewatching.unwrap_or(false) || l.is_rereading.unwrap_or(false);
        let status = parse_mal_list_status(l.status.as_deref(), is_repeating);
        let repeat_count = l
            .num_times_rewatched
            .or(l.num_times_reread)
            .map(|v| v as i32);
        let progress = l
            .num_episodes_watched
            .map(|v| v as i32)
            .or(l.num_chapters_read.map(|v| v as i32));

        ListEntry {
            status,
            score: l.score.map(|v| v as f32),
            progress,
            progress_volumes: l.num_volumes_read.map(|v| v as i32),
            is_repeating,
            repeat_count,
            tags: l.tags.unwrap_or_default(),
            notes: l.comments,
            updated_at: l.updated_at,
        }
    }
}

impl TryFrom<MalItem> for MediaEntry {
    type Error = ();

    fn try_from(item: MalItem) -> Result<Self, Self::Error> {
        let node = item.node;
        let title_str = node.title.ok_or(())?;
        let provider_id = node.id.ok_or(())?.to_string();

        let list_status = item.list_status.or(node.my_list_status);
        let list_entry = list_status.map(ListEntry::from).unwrap_or_default();

        let (english, native) = match node.alternative_titles {
            Some(alt) => (alt.en, alt.ja),
            None => (None, None),
        };

        let (medium, large) = match node.main_picture {
            Some(pic) => (pic.medium, pic.large),
            None => (None, None),
        };

        let extra_large = large.clone().or_else(|| medium.clone());

        let genres = node
            .genres
            .map(|list| list.into_iter().filter_map(|g| g.name).collect())
            .unwrap_or_default();

        let format = parse_mal_media_format(node.media_type.as_deref());
        let media_type = match format {
            MediaFormat::Manga
            | MediaFormat::Novel
            | MediaFormat::OneShot
            | MediaFormat::Doujinshi
            | MediaFormat::Manhwa
            | MediaFormat::Manhua
            | MediaFormat::Oel => MediaType::Manga,
            _ => {
                if node.num_chapters.is_some() || node.num_volumes.is_some() {
                    MediaType::Manga
                } else {
                    MediaType::Anime
                }
            }
        };

        let media = Media {
            id: Uuid::now_v7(),
            provider_id,
            provider: MediaProvider::MAL,
            media_type,
            format,
            release_status: parse_mal_release_status(node.status.as_deref()),
            title: MediaTitle {
                romanized: Some(title_str.clone()),
                english,
                native,
                user_preferred: Some(title_str),
            },
            cover: CoverImage {
                extra_large,
                large,
                medium,
                color: None,
            },
            synopsis: node.synopsis,
            mean_score: node.mean.map(|v| v as f32),
            popularity: node.popularity.map(|v| v as i32),
            episodes: node.num_episodes.map(|v| v as i32),
            duration: node.average_episode_duration.map(|v| v as i32),
            chapters: node.num_chapters.map(|v| v as i32),
            volumes: node.num_volumes.map(|v| v as i32),
            genres,
            nsfw: parse_mal_nsfw(node.nsfw.as_deref()),
        };

        Ok(MediaEntry { media, list_entry })
    }
}

impl From<MalResponse> for PaginatedResponse {
    fn from(res: MalResponse) -> Self {
        let entries: Vec<MediaEntry> = res
            .data
            .into_iter()
            .filter_map(|item| item.try_into().ok())
            .collect();

        let (next_cursor, prev_cursor) = match res.paging {
            Some(p) => (p.next, p.previous),
            None => (None, None),
        };

        let has_next = next_cursor.is_some();

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
