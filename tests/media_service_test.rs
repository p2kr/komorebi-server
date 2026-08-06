use komorebi_server::{
    adapters::MediaClientParams,
    core::{AppError, AppState},
    db::init_db,
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
