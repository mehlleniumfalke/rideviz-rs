use axum::{body::to_bytes, http::Request, Router};
use rideviz_rs::{config::Config, routes, state::AppState};
use tower::ServiceExt;

fn app() -> Router {
    let config = Config::from_env();
    let state = AppState::new(config);
    Router::new()
        .merge(routes::health::router())
        .merge(routes::upload::router())
        .merge(routes::visualize::router())
        .with_state(state)
}

#[tokio::test]
async fn health_returns_ok() {
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .method("GET")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let text = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(text.contains("\"status\":\"ok\""));
}
