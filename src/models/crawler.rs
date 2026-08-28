use std::collections::HashSet;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::media::MediaType;

/// Template Variables:
/// - {query} - title of anime/manga + episode/chapter of anime/manga
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct CrawlerConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub item_selector: String,
    pub title_selector: String,
    pub link_selector: String,
    pub popularity_selector: Option<String>,
    pub size_selector: Option<String>,
    pub is_active: bool,
    pub category: HashSet<MediaType>,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            id: "".into(),
            name: "".into(),
            base_url: "".into(),
            item_selector: "".into(),
            title_selector: "".into(),
            link_selector: "".into(),
            popularity_selector: None,
            size_selector: None,
            is_active: true,
            category: HashSet::from([MediaType::Anime]),
        }
    }
}

impl CrawlerConfig {
    /// Returns a default crawler configuration for Nyaa.si anime torrents.
    /// Last updated on 23-Aug-2026
    pub fn fallback() -> Self {
        Self {
            id: "nyaa".into(),
            name: "Nyaa.si Anime Torrents".into(),
            base_url: "https://nyaa.si/?f=0&c=1_2&q={title}+{number}".into(),
            item_selector: "table.torrent-list tbody tr".into(),
            title_selector: "table.torrent-list tbody tr".into(),
            link_selector: "a[href^='/view/']:not(.comments)".into(),
            popularity_selector: Some("td:nth-child(6)".into()),
            size_selector: Some("td:nth-child(4)".into()),
            is_active: true,
            category: HashSet::from([MediaType::Anime]),
        }
    }
}

#[derive(Serialize, Deserialize, Default, TS)]
#[serde(default)]
#[ts(export)]
pub struct CrawlerResult {
    pub title: String,
    pub link: String,
    pub source: String,
    pub popularity: Option<String>,
    pub size: Option<String>,
    pub parsed_title: ParsedTitle,
    pub category: MediaType,
}

#[derive(Serialize, Deserialize, Default, TS)]
#[ts(export)]
pub struct ParsedTitle {
    pub audio_term: IndexSet<String>,
    pub device: IndexSet<String>,
    pub episode: IndexSet<String>,
    pub episode_title: IndexSet<String>,
    pub file_checksum: IndexSet<String>,
    pub file_extension: IndexSet<String>,
    pub language: IndexSet<String>,
    pub other: IndexSet<String>,
    pub part: IndexSet<String>,
    pub release_group: IndexSet<String>,
    pub release_information: IndexSet<String>,
    pub release_version: IndexSet<String>,
    pub season: IndexSet<String>,
    pub source: IndexSet<String>,
    pub subtitles: IndexSet<String>,
    pub title: IndexSet<String>,
    pub video_resolution: IndexSet<String>,
    pub video_term: IndexSet<String>,
    pub volume: IndexSet<String>,
    pub year: IndexSet<String>,
    pub episode_alt: IndexSet<String>,
    pub date: IndexSet<String>,

    #[serde(rename = "type")]
    pub kind: IndexSet<String>,
}
