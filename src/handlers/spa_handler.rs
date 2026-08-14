use axum::response::IntoResponse;
use reqwest::header;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../komorebi-web/build/"]
struct Assets;

// Axum handler: serve embedded file or fallback to index.html
pub async fn spa_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], file.data).into_response()
        }
        None => {
            // SPA fallback
            let index = Assets::get("index.html").unwrap();
            ([(header::CONTENT_TYPE, "text/html")], index.data).into_response()
        }
    }
}
