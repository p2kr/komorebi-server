use serde::Deserialize;
use uuid::Uuid;

use crate::models::{
    media::{
        MediaCoverImage, MediaItem, MediaListStatus, MediaProvider, MediaTitle, MediaType,
        PaginatedResponse, PagingInfo,
    },
    user::User,
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
    pub rank: Option<i64>,
    pub popularity: Option<i64>,
    pub num_episodes: Option<i64>,
    pub average_episode_duration: Option<i64>,
    pub nsfw: Option<String>,
    pub my_list_status: Option<MalListStatus>,
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
    pub num_volumes_read: Option<i64>,
    pub is_rewatching: Option<bool>,
    pub num_times_rewatched: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub comments: Option<String>,
    pub updated_at: Option<String>,
}

impl From<MalListStatus> for MediaListStatus {
    fn from(l: MalListStatus) -> Self {
        MediaListStatus {
            status: l.status,
            score: l.score.map(|v| v as f32),
            progress: l.num_episodes_watched.map(|v| v as f32),
            progress_volumes: l.num_volumes_read.map(|v| v as i32),
            is_rewatching: l.is_rewatching.unwrap_or(false),
            repeat_count: l.num_times_rewatched.map(|v| v as i32),
            tags: l.tags.unwrap_or_default(),
            comments: l.comments,
            updated_at: l.updated_at,
        }
    }
}

impl TryFrom<MalItem> for MediaItem {
    type Error = ();

    fn try_from(item: MalItem) -> Result<Self, Self::Error> {
        let node = item.node;
        let title_str = node.title.ok_or(())?;

        let list_status = item.list_status.or(node.my_list_status);
        let status = list_status.map(MediaListStatus::from).unwrap_or_default();

        let provider_id = node.id.map(|id| id.to_string());

        let (english, native) = match node.alternative_titles {
            Some(alt) => (
                alt.en.unwrap_or_else(|| title_str.clone()),
                alt.ja.unwrap_or_default(),
            ),
            None => (title_str.clone(), String::new()),
        };

        let (medium, large) = match node.main_picture {
            Some(pic) => (
                pic.medium.unwrap_or_default(),
                pic.large.unwrap_or_default(),
            ),
            None => (String::new(), String::new()),
        };

        let extra_large = if !large.is_empty() {
            large.clone()
        } else {
            medium.clone()
        };

        let genres = node
            .genres
            .map(|list| list.into_iter().filter_map(|g| g.name).collect())
            .unwrap_or_default();

        Ok(MediaItem {
            id: Uuid::now_v7(),
            provider_id,
            provider: MediaProvider::MAL,
            user: User::default(),
            title: MediaTitle {
                romanized: title_str.clone(),
                english,
                native,
                user_preferred: title_str,
            },
            cover_img: MediaCoverImage {
                extra_large,
                large,
                medium,
                color: None,
            },
            synopsis: node.synopsis,
            status,
            media_type: MediaType::ANIME,
            mean_score: node.mean.map(|v| v as f32),
            rank: node.rank.map(|v| v as i32),
            popularity: node.popularity.map(|v| v as i32),
            episodes: node.num_episodes.map(|v| v as i32),
            duration: node.average_episode_duration.map(|v| v as i32),
            genres,
            is_nsfw: node.nsfw.as_deref().map(|s| s != "white").unwrap_or(false),
            ..Default::default()
        })
    }
}

impl From<MalResponse> for PaginatedResponse {
    fn from(res: MalResponse) -> Self {
        let per_page = res.data.len() as i32;
        let media_items: Vec<MediaItem> = res
            .data
            .into_iter()
            .filter_map(|item| item.try_into().ok())
            .collect();

        let (next, prev) = match res.paging {
            Some(p) => (p.next, p.previous),
            None => (None, None),
        };

        PaginatedResponse {
            data: media_items,
            paging_info: PagingInfo {
                has_next: next.is_some(),
                prev,
                next,
                page: 1,
                per_page,
                max_pages: 1,
            },
        }
    }
}
