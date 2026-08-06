use std::{env, sync::LazyLock};

use serde::Deserialize;

#[derive(Clone, PartialEq, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Configs {
    pub mal_client_id: Option<String>,
    pub anilist_client_id: Option<String>,
}

pub static ENV_CONFIGS: LazyLock<Configs> = LazyLock::new(|| Configs {
    mal_client_id: env::var("MAL_CLIENT_ID").ok(),
    anilist_client_id: env::var("ANILIST_CLIENT_ID").ok(),
});
