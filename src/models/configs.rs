use std::{env, sync::LazyLock};

use serde::Deserialize;

#[derive(Clone, PartialEq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Configs {
    mal_client_id: Option<String>,
    mal_client_secret: Option<String>,

    anilist_client_id: Option<String>,
    anilist_client_secret: Option<String>,
}

pub static ENV_CONFIGS: LazyLock<Configs> = LazyLock::new(|| Configs {
    mal_client_id: env::var("MAL_CLIENT_ID").ok(),
    mal_client_secret: env::var("MAL_CLIENT_SECRET").ok(),
    anilist_client_id: env::var("ANILIST_CLIENT_ID").ok(),
    anilist_client_secret: env::var("ANILIST_CLIENT_SECRET").ok(),
});
