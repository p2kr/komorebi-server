use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use reqwest::Url;

use crate::{
    crawlers::Crawler,
    models::crawler::{CrawlerConfig, CrawlerResult},
};

#[derive(Default)]
pub struct HtmlCrawler;

/// Default CSS selectors tried in order when no `title_selector` is configured.
const DEFAULT_TITLE_SELECTORS: &[&str] = &["h1", "h2", ".title", ".name", "a"];

/// Default CSS selectors tried in order when no `link_selector` is configured.
const DEFAULT_LINK_SELECTORS: &[&str] =
    &[r#"a[href^="magnet:"]"#, r#"a[href$=".torrent"]"#, "a[href]"];

/// Default CSS selectors tried in order when no `size_selector` is configured.
const DEFAULT_SIZE_SELECTORS: &[&str] = &[".size", ".filesize", "td.size"];

/// Default CSS selectors tried in order when no `popularity_selector` is configured.
const DEFAULT_POPULARITY_SELECTORS: &[&str] = &[".seeders", ".seeds", "td.seeders"];

fn html_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^\s*(<!doctype\s+html>|<html)").unwrap())
}

/// Collect trimmed text from all of an element's text nodes, joined.
fn element_text(el: scraper::ElementRef<'_>) -> String {
    el.text()
        .map(|t| t.trim())
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .into()
}

/// Parse a slice of CSS selector strings, silently dropping any that are invalid.
fn parse_selectors(strs: &[&str]) -> Vec<scraper::Selector> {
    strs.iter()
        .filter_map(|s| scraper::Selector::parse(s).ok())
        .collect()
}

/// Use the configured selector when non-empty and valid; otherwise fall back to defaults.
fn build_selectors(configured: &str, defaults: &[&str]) -> Vec<scraper::Selector> {
    if !configured.is_empty()
        && let Ok(sel) = scraper::Selector::parse(configured)
    {
        return vec![sel];
    }
    parse_selectors(defaults)
}

impl Crawler for HtmlCrawler {
    fn can_crawl(content: &str) -> bool {
        let sample_size = content.len().min(500);
        html_regex().is_match(&content[..sample_size])
    }

    fn crawl(content: &str, config: &CrawlerConfig) -> Vec<CrawlerResult> {
        let mut results = Vec::new();

        if content.is_empty() || !config.is_active {
            return results;
        }

        // Empty/invalid base_url is valid — sources that only emit absolute links
        // don't need one. Relative links collapse to "" when base_url is absent.
        let base_url = Url::parse(&config.base_url).ok();

        let doc = scraper::Html::parse_document(content);

        // item_selector is required; bail if missing or invalid.
        let Ok(item_selector) = scraper::Selector::parse(&config.item_selector) else {
            return results;
        };

        let title_selectors = build_selectors(&config.title_selector, DEFAULT_TITLE_SELECTORS);
        let link_selectors = build_selectors(&config.link_selector, DEFAULT_LINK_SELECTORS);
        let size_selectors = build_selectors(
            config.size_selector.as_deref().unwrap_or(""),
            DEFAULT_SIZE_SELECTORS,
        );
        let popularity_selectors = build_selectors(
            config.popularity_selector.as_deref().unwrap_or(""),
            DEFAULT_POPULARITY_SELECTORS,
        );

        // Node ID sets used to detect when the item element itself matches the
        // title/link selector (e.g. when title_selector == item_selector).
        // ElementRef doesn't implement Hash, so we key by NodeId.
        let matched_title_ids: HashSet<_> = title_selectors
            .iter()
            .flat_map(|sel| doc.select(sel).map(|e| e.id()))
            .collect();
        let matched_link_ids: HashSet<_> = link_selectors
            .iter()
            .flat_map(|sel| doc.select(sel).map(|e| e.id()))
            .collect();

        for el in doc.select(&item_selector) {
            // ── title ─────────────────────────────────────────────────────────
            // 1. First child matching any title selector
            // 2. Item element itself if it appears in the global title set
            // 3. Full item text as last resort
            let title = {
                let child = title_selectors
                    .iter()
                    .find_map(|sel| el.select(sel).next())
                    .or_else(|| matched_title_ids.contains(&el.id()).then_some(el));
                match child {
                    Some(n) => element_text(n),
                    None => element_text(el),
                }
            };

            // ── link ──────────────────────────────────────────────────────────
            let link_el = link_selectors
                .iter()
                .find_map(|sel| el.select(sel).next())
                .or_else(|| matched_link_ids.contains(&el.id()).then_some(el));

            let mut download_link = match link_el {
                Some(le) => {
                    if let Some(href) = le.attr("href").or_else(|| le.attr("url")) {
                        href.into()
                    } else {
                        let link_text = element_text(le);
                        if !link_text.is_empty() {
                            link_text
                        } else {
                            // Walk parent's raw child nodes by index to catch
                            // bare text nodes between siblings
                            le.parent()
                                .and_then(|parent| {
                                    let children: Vec<_> = parent.children().collect();
                                    let pos = children.iter().position(|c| c.id() == le.id())?;
                                    children
                                        .get(pos + 1)
                                        .and_then(|n| n.value().as_text().map(|t| t.trim().into()))
                                })
                                .unwrap_or_default()
                        }
                    }
                }
                None => String::new(),
            };

            // Only resolve non-empty relative links. `Url::join("")` resolves to
            // the base URL itself, which would bypass the empty guard below.
            if !download_link.is_empty()
                && !download_link.starts_with("http")
                && !download_link.starts_with("magnet")
            {
                download_link = base_url
                    .as_ref()
                    .and_then(|base| base.join(&download_link).ok())
                    .map(|u| u.into())
                    .unwrap_or_default();
            }

            if title.is_empty() && download_link.is_empty() {
                continue;
            }

            let popularity = popularity_selectors.iter().find_map(|sel| {
                let text = element_text(el.select(sel).next()?);
                (!text.is_empty()).then_some(text)
            });

            let size = size_selectors.iter().find_map(|sel| {
                let text = element_text(el.select(sel).next()?);
                (!text.is_empty()).then_some(text)
            });

            results.push(CrawlerResult {
                title,
                link: download_link,
                source: config.id.clone(),
                popularity,
                size,
                ..Default::default()
            });
        }

        results
    }
}
