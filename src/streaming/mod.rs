// pub mod image;
// pub mod novel;
pub mod daemon;
pub mod processor;
pub mod video;

use std::sync::Arc;

use cached::cached;
use camino::Utf8PathBuf;
use itertools::Itertools;
use loco_rs::Result;
use phf::{Map, phf_map};
use serde::Serialize;
use tokio::sync::broadcast::Sender;
use tokio::sync::broadcast::error::SendError;
use ts_rs::TS;
use uuid::Uuid;

use crate::dtos::events::AppEvent;
use crate::dtos::media::MediaType;
use crate::dtos::vault::VaultSubItemDto;
use crate::streaming::processor::MediaProcessor;

pub trait PostProcessor {
    fn post_process(
        processor: Arc<MediaProcessor>,
        item: &VaultSubItemDto,
    ) -> impl Future<Output = Result<()>>;
}

const EXT_VS_TYPE: Map<&str, MediaType> = phf_map! {
    "mkv" | "mp4" | "av1" => MediaType::Anime,
    "png" | "pdf" | "webp" => MediaType::Manga,
};

pub trait ToUrlString {
    fn to_url_string(&self) -> String;
}

impl ToUrlString for Utf8PathBuf {
    fn to_url_string(&self) -> String {
        cached_to_url_string(self)
    }
}

#[cached(max_size = 100)]
fn cached_to_url_string(path: &Utf8PathBuf) -> String {
    path.components().join("/")
}

#[derive(Serialize, Clone, TS)]
pub enum StreamingEvent {
    Init {
        vault_id: Uuid,
        total: usize,
    },
    Progress {
        sub_item_id: Uuid,
        total_size: i64,
        speed: i64,
        progress: f64,
        eta_secs: Option<i64>,
    },
    Complete {
        sub_item_id: Uuid,
        vault_id: Uuid,
        is_last: bool,
    },
}

impl From<StreamingEvent> for AppEvent {
    fn from(value: StreamingEvent) -> Self {
        AppEvent::StreamingEvents(value)
    }
}

impl StreamingEvent {
    pub fn send(self, tx: &Sender<AppEvent>) -> Result<usize, SendError<AppEvent>> {
        tx.send(self.into())
    }
}
