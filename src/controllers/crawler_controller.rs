use std::sync::Arc;

use loco_rs::prelude::*;
use serde::Deserialize;

use crate::{
    controllers::success,
    crawlers::{config_parser::CRAWLER_CONFIGS, crawler_engine::CrawlerEngine},
    models::{crawler::CrawlerConfig, media::MediaType},
};

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct SearchMediaParam {
    query: String,
    media_type: MediaType,
}

#[axum::debug_handler]
pub async fn search_media(
    State(ctx): State<AppContext>,
    Json(params): Json<SearchMediaParam>,
) -> Result<Response> {
    // Extract media_type supporting configs
    let configs: Vec<Arc<CrawlerConfig>> = CRAWLER_CONFIGS
        .iter()
        .filter(|f| f.is_active && f.category.contains(&params.media_type))
        .cloned()
        .collect();

    let client = ctx.shared_store.get().unwrap();
    let engine = CrawlerEngine::new(&params.query, configs, client);

    success(engine.start().await)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("crawler")
        .add("/search", post(search_media))
}
