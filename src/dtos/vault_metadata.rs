use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct Chapter {
    pub id: i64,
    pub title: String,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct Subtitle {
    pub track: usize,
    pub lang: String,
    pub label: String,
    pub format: String,
    pub path: String,
    pub is_forced: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct AudioTrack {
    pub lang: String,
    pub label: String,
    pub channels: String,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct VideoItem {
    pub id: Uuid,
    pub file_name: String,
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub audio_tracks: Vec<AudioTrack>,
    pub subtitles: Vec<Subtitle>,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct VaultMetadata {
    pub id: Uuid,
    pub videos: Vec<VideoItem>,
}
