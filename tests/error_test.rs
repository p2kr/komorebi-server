use axum::http::StatusCode;
use komorebi_server::core::AppError;
use uuid::Uuid;

#[test]
fn test_app_error_status_codes_and_codes() {
    let user_id = Uuid::now_v7();
    let err = AppError::UserNotFound(user_id);
    assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    assert_eq!(err.error_code(), "USER_NOT_FOUND");
    assert_eq!(
        err.to_string(),
        format!("User with ID '{user_id}' not found")
    );

    let upstream_err = AppError::UpstreamApi {
        provider: "MAL".to_string(),
        message: "HTTP status 500 Internal Server Error".to_string(),
    };
    assert_eq!(upstream_err.status_code(), StatusCode::BAD_GATEWAY);
    assert_eq!(upstream_err.error_code(), "UPSTREAM_API_ERROR");
}
