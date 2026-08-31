use komorebi_server::{
    crawlers::{Crawler, html_crawler::HtmlCrawler},
    models::crawler::CrawlerConfig,
};

fn config(base_url: &str) -> CrawlerConfig {
    CrawlerConfig {
        id: "test_source".into(),
        base_url: base_url.into(),
        item_selector: "table tbody tr".into(),
        title_selector: "td.title a".into(),
        link_selector: "td.download a".into(),
        popularity_selector: Some("td.seeders".into()),
        size_selector: Some("td.size".into()),
        ..Default::default()
    }
}

const SAMPLE_HTML: &str = r#"<!DOCTYPE html>
<html>
<body>
<table>
  <tbody>
    <tr>
      <td class="title"><a>Demon Slayer S01</a></td>
      <td class="download"><a href="/view/123">torrent</a></td>
      <td class="size">700 MB</td>
      <td class="seeders">120</td>
    </tr>
    <tr>
      <td class="title"><a>Attack on Titan S04</a></td>
      <td class="download"><a href="magnet:?xt=urn:btih:abc">magnet</a></td>
      <td class="size">1.2 GB</td>
      <td class="seeders">88</td>
    </tr>
    <tr>
      <!-- Gap 6: both title and link empty → should be discarded -->
    </tr>
  </tbody>
</table>
</body>
</html>"#;

#[tokio::test]
async fn test_basic_extraction() {
    let config = config("https://example.com");
    let results = HtmlCrawler::crawl(SAMPLE_HTML, &config);

    // Empty row is discarded (Gap 6)
    assert_eq!(results.len(), 2);

    // Gap 12: source is config.id, not the full base URL
    assert_eq!(results[0].source, "test_source");

    // Gap 8: relative link resolved against base_url
    assert_eq!(results[0].title, "Demon Slayer S01");
    assert_eq!(results[0].link, "https://example.com/view/123");
    assert_eq!(results[0].size.as_deref(), Some("700 MB"));
    assert_eq!(results[0].popularity.as_deref(), Some("120"));

    // Absolute magnet link passed through unchanged
    assert_eq!(results[1].title, "Attack on Titan S04");
    assert_eq!(results[1].link, "magnet:?xt=urn:btih:abc");
}

#[tokio::test]
async fn test_empty_base_url_absolute_links_still_work() {
    // Gap 8: no base_url — absolute magnet links should still be returned
    let config = config("");
    let results = HtmlCrawler::crawl(SAMPLE_HTML, &config);

    // Row 0 has a relative link → collapses to "" → discarded with empty title? No:
    // title is "Demon Slayer S01" (non-empty), so it is kept with an empty link.
    // Row 1 has absolute magnet → kept intact.
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].link, ""); // relative link, no base → empty
    assert_eq!(results[1].link, "magnet:?xt=urn:btih:abc");
}
