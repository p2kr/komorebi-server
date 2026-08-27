use std::sync::Arc;

use reqwest::{Client, Url};

use crate::{
    crawlers::{
        anitomy_title_parser::AnitomyTitleParser, config_parser::CRAWLER_CONFIGS,
        html_crawler::HtmlCrawler, json_crawler::JsonCrawler, Crawler, TitleParser,
    },
    models::crawler::{CrawlerConfig, CrawlerResult, ParsedTitle},
};

pub struct CrawlerEngine {
    query: String,
    configs: Vec<Arc<CrawlerConfig>>,
}

impl CrawlerEngine {
    #[must_use]
    pub fn new(query: &str, configs: Vec<Arc<CrawlerConfig>>) -> Self {
        Self {
            query: query.into(),
            configs: if configs.is_empty() {
                CRAWLER_CONFIGS.clone()
            } else {
                configs
            },
        }
    }

    pub async fn start(&self) -> Vec<CrawlerResult> {
        let mut crawled_res = self.crawl().await;

        Self::parse_title(&mut crawled_res).await;

        crawled_res
    }

    // TODO: Make it robust/dynamic by iterating over list of crawlers
    async fn get_crawl_result(content: &str, config: &CrawlerConfig) -> Vec<CrawlerResult> {
        if JsonCrawler::can_crawl(content) {
            JsonCrawler::crawl(content, config).await
        } else if HtmlCrawler::can_crawl(content) {
            HtmlCrawler::crawl(content, config).await
        } else {
            vec![]
        }
    }

    // TODO: Make it robust/dynamic by iterating over list of parsers
    fn get_title_parser_result(title: &str) -> ParsedTitle {
        AnitomyTitleParser::parse(title)
    }

    async fn crawl(&self) -> Vec<CrawlerResult> {
        let mut results = Vec::new();

        // TODO: Make it async
        for config in &self.configs {
            if !config.is_active {
                continue;
            }

            let url_str = config.base_url.replace("{query}", &self.query);

            let url = match Url::parse(&url_str) {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!("invalid base url for {}. {}", config.id, e);
                    continue;
                }
            };

            // create client
            let client = match Client::builder().build() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("unable to init http client: {}", e);
                    continue;
                }
            };

            // fetch html page
            let resp = match client.get(url.as_ref()).send().await {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("error getting {}. {}", url, e);
                    continue;
                }
            };

            if !resp.status().is_success() {
                tracing::warn!("response: {:?}", resp.status());
                continue;
            }

            if let Ok(content) = resp.text().await {
                let mut result = Self::get_crawl_result(&content, config).await;

                results.append(&mut result);
            }
        }

        results
    }

    // TODO: Make it async
    async fn parse_title(crawl_res: &mut Vec<CrawlerResult>) {
        for res in crawl_res {
            res.parsed_title = Self::get_title_parser_result(&res.title);
        }
    }
}
