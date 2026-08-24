use komorebi_server::{
    crawlers::{json_crawler::JsonCrawler, Crawler},
    models::crawler::CrawlerConfig,
};

#[tokio::test]
async fn test_flat_array_extracts_title_and_link() {
    let content = r#"[
        {
            "title": "Demon Slayer S01",
            "magnet": "magnet:?xt=urn:btih:abc123",
            "size": "700 MB",
            "seeders": "120"
        },
        {
            "name": "Attack on Titan S04",
            "download": "magnet:?xt=urn:btih:def456"
        }
    ]"#;

    let config = CrawlerConfig {
        id: "test_source".into(),
        item_selector: "json".into(),
        ..Default::default()
    };

    let results = JsonCrawler::crawl(content, &config).await;

    assert_eq!(results.len(), 2);

    let first = &results[0];
    assert_eq!(first.title, "Demon Slayer S01");
    assert_eq!(first.link, "magnet:?xt=urn:btih:abc123");
    assert_eq!(first.source, "test_source");
    assert_eq!(first.size.as_deref(), Some("700 MB"));
    assert_eq!(first.popularity.as_deref(), Some("120"));

    let second = &results[1];
    assert_eq!(second.title, "Attack on Titan S04");
    assert_eq!(second.link, "magnet:?xt=urn:btih:def456");
}
