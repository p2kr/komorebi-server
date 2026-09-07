use std::path::PathBuf;

use axum::{body::Body, http::Request};
use cached::concurrent_cached;
use loco_rs::prelude::*;
use serde::Deserialize;
use tokio::fs;
use tower_http::services::ServeFile;
use ts_rs::TS;

use crate::{
    core::constants::{ENCODED_LOC, FONTS_LOC, METADATA_LOC, SUBTITLES_LOC},
    loco_err,
    models::{
        media::MediaType,
        vault::{self},
    },
    streaming::processor::cached_resolve_file_paths,
};

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct VaultStreamPayload {
    pub id: Uuid,
    pub kind: StreamKind,
    pub track: Option<usize>,
    pub name: Option<String>,
}
#[derive(Deserialize, TS, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StreamKind {
    #[default]
    Video,
    Subtitle,
    Font,
    Metadata,
}

pub async fn stream(
    State(ctx): State<AppContext>,
    Query(params): Query<VaultStreamPayload>,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
    let item = vault::Entity::find_by_id(params.id)
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;

    let file_path = match params.kind {
        StreamKind::Video => get_video_path(item.destination_path).await?,
        StreamKind::Font => get_fonts_path(item.destination_path, params.name).await?,
        StreamKind::Subtitle => get_subtitle_path(item.destination_path, params.name).await?,
        StreamKind::Metadata => get_metadata_path(item.destination_path).await?,
    };

    let mut sf = ServeFile::new(file_path);

    let resp = sf.try_call(req).await?;

    Ok(resp)
}

#[concurrent_cached]
async fn get_video_path(dest_path: String) -> Result<PathBuf> {
    let files = cached_resolve_file_paths(&dest_path).await;
    let (path, _) = files
        .into_iter()
        .find(|(_, media_type)| *media_type == MediaType::Anime)
        .ok_or(Error::NotFound)?;
    Ok(path)
}

#[concurrent_cached]
async fn get_fonts_path(dest_path: String, font_name: Option<String>) -> Result<PathBuf> {
    let fonts_path = PathBuf::from(dest_path)
        .join(ENCODED_LOC.as_str())
        .join(FONTS_LOC);

    let mut entries = fs::read_dir(&fonts_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        match font_name.as_ref() {
            Some(f) => {
                if path.is_file() && path.file_name().is_some_and(|v| v == f.as_str()) {
                    return Ok(path);
                }
            }
            None => {
                if path.is_file() && path.ends_with(".ttf")
                    || path.ends_with(".woff")
                    || path.ends_with(".otf")
                {
                    return Ok(path);
                }
            }
        }
    }

    loco_err!("no fonts found")
}

#[concurrent_cached]
async fn get_subtitle_path(dest_path: String, subtitle_name: Option<String>) -> Result<PathBuf> {
    let subtitles_path = PathBuf::from(dest_path)
        .join(ENCODED_LOC.as_str())
        .join(SUBTITLES_LOC);

    let mut entries = fs::read_dir(&subtitles_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        match subtitle_name.as_ref() {
            Some(f) => {
                if path.is_file() && path.file_name().is_some_and(|v| v == f.as_str()) {
                    return Ok(path);
                }
            }
            None => {
                if path.is_file() && path.ends_with(".ass")
                    || path.ends_with(".srt")
                    || path.ends_with(".vtt")
                {
                    return Ok(path);
                }
            }
        }
    }

    loco_err!("no subtitle found")
}

#[concurrent_cached]
async fn get_metadata_path(dest_path: String) -> Result<PathBuf> {
    let metadata_path = PathBuf::from(dest_path)
        .join(ENCODED_LOC.as_str())
        .join(METADATA_LOC);

    if fs::try_exists(&metadata_path).await? {
        return Ok(metadata_path);
    }

    loco_err!("no metadata found")
}
