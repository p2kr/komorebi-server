use std::sync::Arc;

use loco_rs::prelude::*;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use reqwest::{Client, Url};
use tokio::task::JoinSet;
use url::form_urlencoded;

use crate::{
    core::ResultExt,
    crawlers::{
        Crawler, TitleParser, anitomy_title_parser::AnitomyTitleParser,
        config_parser::CRAWLER_CONFIGS, html_crawler::HtmlCrawler, json_crawler::JsonCrawler,
    },
    loco_err, loco_err_msg,
    models::crawler::{CrawlerConfig, CrawlerResult, ParsedTitle},
};

pub struct CrawlerEngine {
    query: String,
    configs: Vec<Arc<CrawlerConfig>>,
    client: Client,
}

impl CrawlerEngine {
    #[must_use]
    pub fn new(query: &str, configs: Vec<Arc<CrawlerConfig>>, client: Client) -> Self {
        Self {
            query: query.into(),
            configs: if configs.is_empty() {
                CRAWLER_CONFIGS.clone()
            } else {
                configs
            },
            client,
        }
    }

    pub async fn start(&self) -> Vec<CrawlerResult> {
        let crawled_res = self.crawl().await;

        Self::parse_title(crawled_res).await
    }

    // TODO: Make it robust/dynamic by iterating over list of crawlers
    fn get_crawl_result(content: &str, config: &CrawlerConfig) -> Vec<CrawlerResult> {
        if JsonCrawler::can_crawl(content) {
            JsonCrawler::crawl(content, config)
        } else if HtmlCrawler::can_crawl(content) {
            HtmlCrawler::crawl(content, config)
        } else {
            Default::default()
        }
    }

    // TODO: Make it robust/dynamic by iterating over list of parsers
    fn get_title_parser_result(title: &str) -> ParsedTitle {
        if AnitomyTitleParser::can_parse(title) {
            AnitomyTitleParser::parse(title)
        } else {
            Default::default()
        }
    }

    async fn crawl(&self) -> Vec<CrawlerResult> {
        let mut results = Vec::new();
        let mut set: JoinSet<Result<Vec<CrawlerResult>>> = JoinSet::new();

        for config in &self.configs {
            if !config.is_active {
                continue;
            }

            let encoded_query: String =
                form_urlencoded::byte_serialize(self.query.as_bytes()).collect();

            let url_str = config.base_url.replace("{query}", &encoded_query);

            let url = match Url::parse(&url_str) {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!("invalid base url for {}. {}", config.id, e);
                    continue;
                }
            };

            let bg_client = self.client.clone();
            let bg_config = config.clone();
            set.spawn(async move {
                tracing::debug!("starting crawl on {}", url_str);
                // fetch html page
                let resp = bg_client
                    .get(url.as_ref())
                    .send()
                    .await
                    .map_err(|e| loco_err_msg!("error getting {}. {}", url, e))?;

                if !resp.status().is_success() {
                    return loco_err!("response: {:?}", resp.status());
                }

                let content = resp.text().await.to_loco_err()?;

                tracing::debug!("downloaded page {}. starting parse...", url_str);

                tokio::task::spawn_blocking(move || {
                    Ok(Self::get_crawl_result(&content, bg_config.as_ref()))
                })
                .await
                .inspect(|_| tracing::debug!("parsing finished for {}", url_str))
                .to_loco_err()?
            });
        }

        while let Some(res) = set.join_next().await {
            match res {
                Err(e) => tracing::error!("Error in joining crawlers: {}", e),
                Ok(r) => match r {
                    Ok(mut res) => results.append(&mut res),
                    Err(e) => tracing::error!("{}", e),
                },
            }
        }

        results
    }

    async fn parse_title(mut crawl_res: Vec<CrawlerResult>) -> Vec<CrawlerResult> {
        tokio::task::spawn_blocking(move || {
            crawl_res.par_iter_mut().for_each(|res| {
                res.parsed_title = Self::get_title_parser_result(&res.title);
            });
            crawl_res
        })
        .await
        .unwrap_or_default()
    }
}
