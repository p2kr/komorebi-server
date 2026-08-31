use std::time::Duration;

use loco_rs::prelude::*;
use reqwest::Client;

use crate::loco_err_msg;

/// This client has timeout and should not be used for long running requests.
/// Use for connecting to providers (MAL/Anilist)
pub fn get_reqwest_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| loco_err_msg!("failed to create http client pool: {}", e))
}

const TRACKERS_URL: &str =
    "https://raw.githubusercontent.com/ngosang/trackerslist/refs/heads/master/trackers_all.txt";

pub async fn get_common_trackers(client: &Client) -> Vec<String> {
    if let Ok(resp) = client.get(TRACKERS_URL).send().await
        && resp.status().is_success()
        && let Ok(text) = resp.text().await
    {
        return text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|t| String::from(t.trim()))
            .collect();
    }
    Vec::new()
}
