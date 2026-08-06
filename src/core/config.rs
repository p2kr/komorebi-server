use std::{env, sync::LazyLock};

use serde::Deserialize;
use tracing::debug;

#[derive(Clone, PartialEq, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Configs {
    pub mal_client_id: Option<String>,
    pub anilist_client_id: Option<String>,
    pub cors_origins: Vec<String>,
}

pub static ENV_CONFIGS: LazyLock<Configs> = LazyLock::new(|| {
    let mut config = Configs {
        mal_client_id: env::var("MAL_CLIENT_ID").ok(),
        anilist_client_id: env::var("ANILIST_CLIENT_ID").ok(),
        ..Default::default()
    };
    if let Ok(origin) = env::var("CORS_ORIGINS") {
        let origins: Vec<String> = origin.split(',').map(|s| s.trim().to_string()).collect();
        config.cors_origins = origins;
        debug!("CORS_ORIGINS: {:?}", config.cors_origins);
    }

    config
});
