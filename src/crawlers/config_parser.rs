use std::{
    fs,
    sync::{Arc, LazyLock},
};

use indexmap::IndexMap;

use crate::models::crawler::CrawlerConfig;

pub static CRAWLER_CONFIGS: LazyLock<Vec<Arc<CrawlerConfig>>> =
    LazyLock::new(|| get_crawler_configs().into_iter().map(Arc::new).collect());

fn get_config_str() -> String {
    let fallback = include_str!("../../assets/crawler_configs.yaml");

    match fs::read_to_string("assets/crawler_configs.yaml") {
        Ok(config_str) => {
            tracing::debug!("loaded crawler configs from assets/crawler_configs.yaml");
            config_str
        }
        Err(e) => {
            tracing::warn!(
               "failed to read crawler configs from assets/crawler_configs.yaml, falling back to default: {}",
               e
           );
            String::from(fallback)
        }
    }
}

pub fn get_crawler_configs() -> Vec<CrawlerConfig> {
    let crawler_config = get_config_str();

    let map = match yaml_serde::from_str::<IndexMap<String, CrawlerConfig>>(&crawler_config) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                "failed to parse crawler configs due to {}, falling back to default",
                e
            );
            return vec![CrawlerConfig::fallback()];
        }
    };

    let keys: Vec<String> = map.keys().cloned().collect();
    tracing::debug!("loaded [{}]{:?} crawler configs", keys.len(), keys);

    map.into_iter()
        .map(|(k, mut v)| {
            v.id = k;
            v
        })
        .collect()
}
