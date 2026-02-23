use axum::{body::to_bytes, http::Request, Router};
use rideviz_rs::{
    config::Config,
    license::create_license_token,
    routes,
    state::AppState,
};
use serde_json::Value;
use tower::ServiceExt;

fn app_with_config(config: Config) -> (Router, AppState) {
    let state = AppState::new(config);
    let app = Router::new().merge(routes::visualize::router()).with_state(state.clone());
    (app, state)
}

fn bearer(secret: &str, user_id: &str, email: &str) -> String {
    let token = create_license_token(user_id, email, true, 3600, secret).expect("token");
    format!("Bearer {token}")
}

#[tokio::test]
async fn export_video_rate_limits_before_not_found() {
    let mut config = Config::default();
    config.video_export_rate_limit_max_requests = 1;
    config.video_export_rate_limit_window = std::time::Duration::from_secs(60);
    config.video_export_max_concurrency = 10;
    let (app, _) = app_with_config(config.clone());

    let request_json = serde_json::json!({
        "file_id": "missing",
        "duration_seconds": 3.0,
        "fps": 24
    });

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/export/video")
                .method("POST")
                .header("content-type", "application/json")
                .header("authorization", bearer(&config.jwt_secret, "u-rate", "rate@example.com"))
                .body(axum::body::Body::from(request_json.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first.status(), axum::http::StatusCode::NOT_FOUND);

    let second = app
        .oneshot(
            Request::builder()
                .uri("/api/export/video")
                .method("POST")
                .header("content-type", "application/json")
                .header("authorization", bearer(&config.jwt_secret, "u-rate", "rate@example.com"))
                .body(axum::body::Body::from(request_json.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);

    let body = to_bytes(second.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(
        json.get("code").and_then(Value::as_str).unwrap_or(""),
        "rate_limited"
    );
    assert!(json.get("request_id").is_some());
}

#[tokio::test]
async fn export_video_returns_busy_when_concurrency_exhausted() {
    let mut config = Config::default();
    config.video_export_max_concurrency = 1;
    config.video_export_queue_timeout = std::time::Duration::from_secs(0);
    config.video_export_rate_limit_max_requests = 1000;
    let (app, state) = app_with_config(config.clone());

    let _held = state
        .video_export_semaphore()
        .acquire_owned()
        .await
        .expect("permit");

    let request_json = serde_json::json!({
        "file_id": "missing",
        "duration_seconds": 3.0,
        "fps": 24
    });

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/export/video")
                .method("POST")
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    bearer(&config.jwt_secret, "u-busy", "busy@example.com"),
                )
                .body(axum::body::Body::from(request_json.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let json: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(
        json.get("code").and_then(Value::as_str).unwrap_or(""),
        "export_busy"
    );
    assert!(json.get("request_id").is_some());
}

