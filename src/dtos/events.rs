use std::convert::Infallible;

use axum::response::sse::Event;
use serde::Serialize;
use strum::AsRefStr;
use tokio::sync::broadcast::{Sender, error::SendError};
use ts_rs::TS;

use crate::{
    models::{vault::VaultItem, vault_sub_item::VaultSubItem},
    streaming::StreamingEvent,
};

#[derive(Clone, Serialize, TS, AsRefStr, Default)]
#[ts(export)]
#[serde(tag = "type", content = "data")] // Creates clean JSON for the frontend
pub enum AppEvent {
    #[default]
    Unknown,
    VaultItems(Vec<VaultItem>),
    VaultSubItems(Vec<VaultSubItem>),
    StreamingEvents(StreamingEvent),
    Error(String),
}

impl AppEvent {
    pub fn to_sse_opt(&self) -> Option<Result<Event, Infallible>> {
        Some(Ok::<_, Infallible>(
            Event::default()
                .event(self.as_ref())
                .json_data(self)
                .unwrap_or_default(),
        ))
    }

    pub fn to_sse(&self) -> Result<Event, Infallible> {
        Ok::<_, Infallible>(
            Event::default()
                .event(self.as_ref())
                .json_data(self)
                .unwrap_or_default(),
        )
    }

    pub fn send(self, tx: &Sender<AppEvent>) -> Result<usize, SendError<AppEvent>> {
        tx.send(self)
    }
}
