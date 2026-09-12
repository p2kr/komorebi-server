use std::path::PathBuf;

use axum::{body::Body, http::Request};
use cached::cached;
use dashmap::DashMap;
use itertools::Itertools;
use loco_rs::prelude::*;
use sea_orm::{DbConn, LoaderTrait};
use serde::Deserialize;
use tokio::fs;
use tokio::task::JoinSet;
use tower_http::services::ServeFile;
use ts_rs::TS;
use walkdir::WalkDir;

use crate::controllers::success;
use crate::core::constants::VAULT_LOC;
use crate::dtos::vault::{VaultMetadataDto, VaultSubItemDto};
use crate::models::{
    audio_tracks, subtitle_fonts, vault_metadata, vault_sub_item, video_chapters, video_subtitles,
};

#[derive(Deserialize, TS)]
#[ts(export)]
pub struct VaultStreamPayload {
    /// Path to streaming file
    pub path: String,
}

#[derive(Clone, Debug, serde::Deserialize, TS)]
#[ts(export)]
pub struct VaultSubItemPayload {
    pub vault_ids: Vec<Uuid>,
}

pub async fn stream(
    State(_ctx): State<AppContext>,
    Query(params): Query<VaultStreamPayload>,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
    let path = PathBuf::from(params.path);
    if fs::try_exists(&path).await? {
        if is_file_in_vault(&path) {
            let mut sf = ServeFile::new(&path);
            let resp = sf.try_call(req).await?;
            Ok(resp)
        } else {
            unauthorized("file is not in vault")
        }
    } else {
        not_found()
    }
}

#[cached(max_size = 100)]
fn is_file_in_vault(file_path: &PathBuf) -> bool {
    let vault_root = PathBuf::from(VAULT_LOC.to_owned());
    for entry in WalkDir::new(vault_root)
        .max_depth(10)
        .follow_links(false)
        .into_iter()
    {
        if let Ok(path) = entry
            && path.file_type().is_file()
            && path.into_path().eq(file_path)
        {
            return true;
        }
    }
    false
}

pub async fn get_metadata(
    State(ctx): State<AppContext>,
    axum::Json(param): axum::Json<VaultSubItemPayload>,
) -> Result<impl IntoResponse> {
    let mut set = JoinSet::new();
    let map = DashMap::new();
    for vault_id in param.vault_ids {
        let db = ctx.db.clone();
        set.spawn(async move { (vault_id, cached_metadata(vault_id, &db).await) });
    }

    while let Some(task) = set.join_next().await {
        if let Ok((id, Ok(value))) = task {
            map.insert(id, value);
        }
    }

    success(map)
}

#[cached(
    ttl_secs = 60, // Expiry in seconds (1 minute)
    key = "String", // The type of the cache key
    sync_writes = "by_key",
    convert = { vault_id.to_string() } // Construct the key (ignoring `db`)
)]
async fn cached_metadata(vault_id: Uuid, db: &DbConn) -> Result<Vec<VaultSubItemDto>> {
    let sub_items = vault_sub_item::Entity::find()
        .filter(vault_sub_item::Column::VaultId.eq(vault_id))
        .find_also_related(vault_metadata::Entity)
        .all(db)
        .await?;

    // Extract all metadata models into a single Vec for the Loader
    let metadatas = sub_items
        .iter()
        .filter_map(|(_, meta_opt)| meta_opt.clone()) // Safely extract only existing metadata
        .collect_vec();

    // Fetch all related tracks, subtitles, fonts, and chapters in BULK (4 queries)
    let all_audio_tracks = metadatas
        .load_many(audio_tracks::Entity, db)
        .await
        .unwrap_or_default();
    let all_video_subtitles = metadatas
        .load_many(video_subtitles::Entity, db)
        .await
        .unwrap_or_default();
    let all_subtitle_fonts = metadatas
        .load_many(subtitle_fonts::Entity, db)
        .await
        .unwrap_or_default();
    let all_video_chapters = metadatas
        .load_many(video_chapters::Entity, db)
        .await
        .unwrap_or_default();

    let mut resp: Vec<VaultSubItemDto> = Vec::with_capacity(sub_items.len());
    let mut meta_idx = 0;
    for (sub_item, meta_opt) in sub_items {
        let dto = if let Some(meta) = meta_opt {
            let curr_idx = meta_idx;
            meta_idx += 1;
            VaultSubItemDto {
                sub_item,
                metadata: Some(VaultMetadataDto {
                    vault_metadata: meta,
                    audio_tracks: all_audio_tracks.get(curr_idx).cloned().unwrap_or_default(),
                    video_subtitles: all_video_subtitles
                        .get(curr_idx)
                        .cloned()
                        .unwrap_or_default(),
                    subtitle_fonts: all_subtitle_fonts
                        .get(curr_idx)
                        .cloned()
                        .unwrap_or_default(),
                    video_chapters: all_video_chapters
                        .get(curr_idx)
                        .cloned()
                        .unwrap_or_default(),
                }),
            }
        } else {
            VaultSubItemDto {
                sub_item,
                metadata: None,
            }
        };
        resp.push(dto);
    }
    Ok(resp)
}
