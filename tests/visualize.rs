use axum::{body::to_bytes, http::Request, Router};
use rideviz_rs::{config::Config, routes, state::AppState};
use serde_json::Value;
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

fn sample_gpx() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <trk><name>Test Ride</name><trkseg>
    <trkpt lat="52.5200" lon="13.4050"><ele>34.0</ele><time>2026-01-01T12:00:00Z</time></trkpt>
    <trkpt lat="52.5205" lon="13.4060"><ele>39.0</ele><time>2026-01-01T12:00:10Z</time></trkpt>
  </trkseg></trk>
</gpx>"#
}

fn multipart_body(file_name: &str, file_body: &str, boundary: &str) -> String {
    format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n{file_body}\r\n--{boundary}--\r\n"
    )
}

#[tokio::test]
async fn visualize_static_png_returns_image() {
    let app = app();
    let boundary = "X-BOUNDARY-TEST";
    let upload_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/upload")
                .method("POST")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(multipart_body("ride.gpx", sample_gpx(), boundary)))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(upload_response.status(), axum::http::StatusCode::OK);
    let upload_body = to_bytes(upload_response.into_body(), usize::MAX)
        .await
        .expect("upload body");
    let upload_json: Value = serde_json::from_slice(&upload_body).expect("upload json");
    let file_id = upload_json
        .get("file_id")
        .and_then(Value::as_str)
        .expect("file id");

    let request_json = serde_json::json!({
        "file_id": file_id,
        "gradient": "fire",
        "width": 1080,
        "height": 1080,
        "background": "transparent"
    });
    let visualize_response = app
        .oneshot(
            Request::builder()
                .uri("/api/visualize")
                .method("POST")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(request_json.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(visualize_response.status(), axum::http::StatusCode::OK);
    let content_type = visualize_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(content_type, "image/png");
    let body = to_bytes(visualize_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    assert!(body.len() > 100);
}
