use serde_json::Value;

use crate::{
    crawlers::Crawler,
    models::crawler::{CrawlerConfig, CrawlerResult},
};

#[derive(Default)]
pub struct JsonCrawler;

// Sentinel value for item_selector that means "treat the whole document as JSON"
const JSON_SELECTOR: &str = "json";

/// Default keys tried in order when looking for the list of items inside a JSON object.
const DEFAULT_ITEM_LIST_KEYS: &[&str] = &[
    "results", "items", "data", "torrents", "list", "entries", "records",
];

/// Default keys tried in order when extracting a title string from an item.
const DEFAULT_TITLE_KEYS: &[&str] = &["title", "name", "display_name", "label"];

/// Default keys tried in order when extracting a download / magnet URL.
const DEFAULT_DOWNLOAD_URL_KEYS: &[&str] = &[
    "magnet",
    "magnet_link",
    "download",
    "url",
    "link",
    "torrent_url",
];

/// Default keys tried in order when looking for a nested list of downloads
/// (e.g. `{ "downloads": [ { "resolution": "1080p", "magnet": "…" } ] }`).
const DEFAULT_DOWNLOAD_LIST_KEYS: &[&str] = &["downloads", "links", "files", "torrents"];

/// Default keys tried in order when extracting a file-size string.
const DEFAULT_SIZE_KEYS: &[&str] = &["size", "file_size", "filesize", "length"];

/// Default keys tried in order when extracting a popularity / seeders value.
const DEFAULT_POPULARITY_KEYS: &[&str] = &["seeders", "seeds", "popularity", "score"];

/// Default keys tried when looking for a video resolution tag inside a download entry.
const DEFAULT_RESOLUTION_KEYS: &[&str] = &["resolution", "quality", "res", "video_quality"];

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Navigate a `serde_json::Value` by a dot-separated path, e.g. `"data.items"`.
fn get_value_by_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

/// Try each key in `keys` in order and return the first `Value` found (any type).
fn get_first_value_by_path_list<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|k| value.get(*k))
}

/// Extract the first non-empty string found under any key in `candidate_keys`.
/// Supports dot-path notation for nested keys.
fn extract_string(value: &Value, candidate_keys: &[&str]) -> String {
    for key in candidate_keys {
        if let Some(found) = get_value_by_path(value, key) {
            let s = match found {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => continue,
            };
            if !s.is_empty() {
                return s;
            }
        }
    }
    String::new()
}

// ─── implementation ───────────────────────────────────────────────────────────

impl Crawler for JsonCrawler {
    /// Returns `true` when the content is valid JSON (object or array).
    fn can_crawl(content: &str) -> bool {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return false;
        }
        matches!(
            serde_json::from_str::<Value>(trimmed),
            Ok(Value::Object(_) | Value::Array(_))
        )
    }

    fn crawl(content: &str, config: &CrawlerConfig) -> Vec<CrawlerResult> {
        let mut results = Vec::new();

        if content.is_empty() || !config.is_active {
            return results;
        }

        let decoded: Value = match serde_json::from_str(content.trim()) {
            Ok(v) => v,
            Err(_) => return results,
        };

        // Collect items — each entry is (Option<key_string>, &Value)
        let mut items_to_process: Vec<(Option<String>, &Value)> = Vec::new();

        match &decoded {
            Value::Array(arr) => {
                for item in arr {
                    items_to_process.push((None, item));
                }
            }
            Value::Object(_) => {
                let custom_path = if config.item_selector.to_lowercase() != JSON_SELECTOR
                    && !config.item_selector.is_empty()
                {
                    Some(config.item_selector.as_str())
                } else {
                    None
                };

                let list_in_map = if let Some(path) = custom_path {
                    get_value_by_path(&decoded, path)
                } else {
                    get_first_value_by_path_list(&decoded, DEFAULT_ITEM_LIST_KEYS)
                };

                if let Some(Value::Array(arr)) = list_in_map {
                    for item in arr {
                        items_to_process.push((None, item));
                    }
                } else if let Value::Object(map) = &decoded {
                    for (k, v) in map {
                        items_to_process.push((Some(k.clone()), v));
                    }
                }
            }
            _ => return results,
        }

        // Build candidate key lists from config (user-supplied key first).
        let title_keys: Vec<&str> = config
            .title_selector
            .is_empty()
            .then_some(DEFAULT_TITLE_KEYS.to_vec())
            .unwrap_or_else(|| {
                let mut v = vec![config.title_selector.as_str()];
                v.extend_from_slice(DEFAULT_TITLE_KEYS);
                v
            });

        let download_keys: Vec<&str> = config
            .link_selector
            .is_empty()
            .then_some(DEFAULT_DOWNLOAD_URL_KEYS.to_vec())
            .unwrap_or_else(|| {
                let mut v = vec![config.link_selector.as_str()];
                v.extend_from_slice(DEFAULT_DOWNLOAD_URL_KEYS);
                v
            });

        let size_keys: Vec<&str> = config
            .size_selector
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|sel| {
                let mut v = vec![sel];
                v.extend_from_slice(DEFAULT_SIZE_KEYS);
                v
            })
            .unwrap_or_else(|| DEFAULT_SIZE_KEYS.to_vec());

        let popularity_keys: Vec<&str> = config
            .popularity_selector
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|sel| {
                let mut v = vec![sel];
                v.extend_from_slice(DEFAULT_POPULARITY_KEYS);
                v
            })
            .unwrap_or_else(|| DEFAULT_POPULARITY_KEYS.to_vec());

        for (item_key, item_data) in &items_to_process {
            let Value::Object(_) = item_data else {
                continue;
            };
            let item_map = *item_data;

            // ── title ──────────────────────────────────────────────────────
            let base_title: String = {
                let from_fields = extract_string(item_map, &title_keys);
                match item_key.as_deref() {
                    Some(k) if from_fields.is_empty() || k.len() > from_fields.len() => {
                        k.trim().into()
                    }
                    _ => from_fields.trim().into(),
                }
            };

            // ── nested download list (e.g. multiple resolutions) ───────────
            let link_selector_val = if !config.link_selector.is_empty() {
                get_value_by_path(item_map, &config.link_selector)
            } else {
                None
            };

            let downloads_list = match link_selector_val {
                Some(Value::Array(_)) => link_selector_val,
                _ => get_first_value_by_path_list(item_map, DEFAULT_DOWNLOAD_LIST_KEYS),
            };

            if let Some(Value::Array(downloads)) = downloads_list {
                for download in downloads {
                    let Value::Object(_) = download else { continue };

                    let res = extract_string(download, DEFAULT_RESOLUTION_KEYS);
                    let magnet = extract_string(download, &download_keys);

                    if magnet.is_empty() {
                        continue;
                    }

                    let res_tag = if !res.is_empty()
                        && !base_title.to_lowercase().contains(&res.to_lowercase())
                    {
                        let res_str = if res.ends_with('p') {
                            res.clone()
                        } else {
                            format!("{res}p")
                        };
                        format!(" ({res_str})")
                    } else {
                        String::new()
                    };

                    let size = extract_string(download, &size_keys);
                    let popularity = extract_string(download, &popularity_keys);

                    results.push(CrawlerResult {
                        title: format!("{base_title}{res_tag}"),
                        link: magnet,
                        source: config.id.clone(),
                        popularity: (!popularity.is_empty()).then_some(popularity),
                        size: (!size.is_empty()).then_some(size),
                        ..Default::default()
                    });
                }
            } else {
                // ── flat item (one download per object) ────────────────────
                let download_url = extract_string(item_map, &download_keys);
                let size = extract_string(item_map, &size_keys);
                let popularity = extract_string(item_map, &popularity_keys);

                if !base_title.is_empty() || !download_url.is_empty() {
                    results.push(CrawlerResult {
                        title: base_title,
                        link: download_url,
                        source: config.id.clone(),
                        popularity: (!popularity.is_empty()).then_some(popularity),
                        size: (!size.is_empty()).then_some(size),
                        ..Default::default()
                    });
                }
            }
        }

        results
    }
}
