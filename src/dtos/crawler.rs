use std::collections::HashSet;

use educe::Educe;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::dtos::MediaType;

/// Template Variables:
/// - {query} - title of anime/manga + episode/chapter of anime/manga
#[derive(Debug, Deserialize, Educe)]
#[educe(Default)]
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

    #[educe(Default = true)]
    pub is_active: bool,

    #[educe(Default = HashSet::from([MediaType::Anime]))]
    pub category: HashSet<MediaType>,
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
            ..Default::default()
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

macro_rules! make_parsed_title {
    ($($field:ident),* $(,)?) => {
        #[derive(Debug, Clone, Serialize, Deserialize, Default, TS, PartialEq)]
        #[ts(export)]
        pub struct ParsedTitle {
            $(
                #[serde(skip_serializing_if = "IndexSet::is_empty")]
                #[ts(type = "string[] | undefined")]
                pub $field: IndexSet<String>,
            )*

            // Manually add `kind` since it needs the extra `rename` attribute
            #[serde(rename = "type", skip_serializing_if = "IndexSet::is_empty")]
            #[ts(type = "string[] | undefined")]
            pub kind: IndexSet<String>,
        }
    };
}

// Just list your fields here. The macro does the rest!
make_parsed_title!(
    audio_term,
    device,
    episode,
    episode_title,
    file_checksum,
    file_extension,
    language,
    other,
    part,
    release_group,
    release_information,
    release_version,
    season,
    source,
    subtitles,
    title,
    video_resolution,
    video_term,
    volume,
    year,
    episode_alt,
    date
);
