pub mod anitomy_title_parser;
pub mod config_parser;
pub mod crawler_engine;
pub mod html_crawler;
pub mod json_crawler;

use async_trait::async_trait;

use crate::models::crawler::{CrawlerConfig, CrawlerResult, ParsedTitle};

#[async_trait]
pub trait Crawler {
    #[must_use]
    fn can_crawl(content: &str) -> bool;
    async fn crawl(content: &str, config: &CrawlerConfig) -> Vec<CrawlerResult>;
}

pub trait TitleParser {
    #[must_use]
    fn can_parse(raw_title: &str) -> bool;
    fn parse(raw_title: &str) -> ParsedTitle;
}
