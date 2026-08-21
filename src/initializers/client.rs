use std::time;

use reqwest::Client;

pub fn get_reqwest_client() -> Client {
    Client::builder()
        .timeout(time::Duration::from_secs(15))
        .build()
        .expect("failed to create http client pool")
}
