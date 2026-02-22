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

fn sample_gpx() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="test">
  <trk><name>Test Ride</name><trkseg>
    <trkpt lat="52.5200" lon="13.4050"><ele>34.0</ele><time>2026-01-01T12:00:00Z</time><extensions><gpxtpx:hr>140</gpxtpx:hr></extensions></trkpt>
    <trkpt lat="52.5205" lon="13.4060"><ele>39.0</ele><time>2026-01-01T12:00:10Z</time><extensions><gpxtpx:hr>145</gpxtpx:hr></extensions></trkpt>
  </trkseg></trk>
</gpx>"#
}

fn multipart_body(file_name: &str, file_body: &str, boundary: &str) -> String {
    format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n{file_body}\r\n--{boundary}--\r\n"
    )
}

#[tokio::test]
async fn upload_gpx_returns_file_id_and_metrics() {
    let boundary = "X-BOUNDARY-TEST";
    let body = multipart_body("ride.gpx", sample_gpx(), boundary);

    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/upload")
                .method("POST")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let text = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(text.contains("\"file_id\""));
    assert!(text.contains("\"available_data\""));
}

#[tokio::test]
async fn upload_rejects_unsupported_extension() {
    let boundary = "X-BOUNDARY-TEST";
    let body = multipart_body("ride.txt", "hello", boundary);

    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/upload")
                .method("POST")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(axum::body::Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
}
