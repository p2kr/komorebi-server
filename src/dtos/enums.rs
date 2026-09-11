use sea_orm::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;
use ts_rs::TS;

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, DeriveActiveEnum, EnumIter, TS,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "UPPERCASE"
)]
pub enum MediaProvider {
    #[default]
    MAL,
    ANILIST,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Default,
    EnumString,
    DeriveActiveEnum,
    EnumIter,
    TS,
)]
#[strum(ascii_case_insensitive)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "UPPERCASE"
)]
pub enum MediaType {
    #[default]
    Anime,
    Manga,
    Novel,
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_str(&s).unwrap_or_default())
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Default,
    DeriveActiveEnum,
    EnumIter,
    TS,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "UPPERCASE"
)]
pub enum VaultDownloadType {
    DIRECT, // HTTP/HTTPS direct download
    #[default]
    MAGNET, // Magnet link
    TFILE,  // Torrent file
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, DeriveActiveEnum, EnumIter, TS,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "UPPERCASE"
)]
pub enum VaultStatus {
    #[default]
    /// added to queue, not yet downloading
    PENDING,
    /// actively downloading
    DOWNLOADING,
    /// user paused
    PAUSED,
    /// download bytes done (internal transient — daemon picks this up)
    COMPLETED,
    /// remux/transcode in progress
    PROCESSING,
    /// stream-ready
    READY,
    /// Partial ready
    PARTIAL,
    /// download or post-process error
    FAILED,
    /// user deleted
    CANCELLED,
}
