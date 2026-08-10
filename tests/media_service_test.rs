use komorebi_server::{
    adapters::MediaClientParams,
    core::{AppError, AppState},
    db::init_db,
    handlers::media_handler::Params,
    models::media::MediaProvider,
    services::media_service::MediaService,
};
use uuid::Uuid;

#[tokio::test]
async fn test_media_service_returns_not_found_for_missing_user() {
    let pool = init_db().await;
    let http_client = reqwest::Client::new();
    let state = AppState {
        db: pool,
        http_client,
    };

    let params = MediaClientParams {
        user_id: Uuid::now_v7(),
        ..Default::default()
    };

    let res = MediaService::get_user_anime_list(&state, &MediaProvider::MAL, &params).await;
    assert!(res.is_err());

    if let Err(AppError::UserNotFound(id)) = res {
        assert_eq!(id, params.user_id);
    } else {
        panic!("expected AppError::UserNotFound");
    }
}

#[test]
fn test_json_body_deserialization() {
    let json_data = serde_json::json!({
        "user_id": "019fd567-0921-7631-abe0-3e2fc1737ea4",
        "provider": "MAL",
        "limit": 50,
        "offset": 0
    });
    let params: Params = serde_json::from_value(json_data).unwrap();

    assert_eq!(params.provider, Some(MediaProvider::MAL));
    assert_eq!(
        params.params.user_id,
        Uuid::parse_str("019fd567-0921-7631-abe0-3e2fc1737ea4").unwrap()
    );
    assert_eq!(params.params.limit, Some(50));
    assert_eq!(params.params.offset, Some(0));
}

#[test]
fn test_json_body_deserialization_defaults() {
    let json_data = serde_json::json!({
        "user_id": "019fd567-0921-7631-abe0-3e2fc1737ea4"
    });
    let params: Params = serde_json::from_value(json_data).unwrap();

    assert_eq!(params.provider, None);
    assert_eq!(
        params.params.user_id,
        Uuid::parse_str("019fd567-0921-7631-abe0-3e2fc1737ea4").unwrap()
    );
    assert_eq!(params.params.limit, None);
    assert_eq!(params.params.offset, None);
}
