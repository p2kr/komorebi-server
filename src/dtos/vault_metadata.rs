use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct ChapterItem {
    pub id: i64,
    pub title: String,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct SubtitleTrackItem {
    pub track: usize,
    pub language: String,
    pub title: String,
    pub format: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Default)]
#[ts(export)]
pub struct VaultMetadata {
    pub chapters: Vec<ChapterItem>,
    pub subtitles: Vec<SubtitleTrackItem>,
    pub fonts: Vec<String>,
}
