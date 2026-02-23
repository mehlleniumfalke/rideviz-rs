use axum::{
    body::{to_bytes, Body},
    http::Request,
    Router,
};
use hmac::{Hmac, Mac};
use rideviz_rs::{config::Config, license::verify_license_token, routes, state::AppState};
use sha2::Sha256;
use tower::ServiceExt;

fn app(config: Config) -> Router {
    let state = AppState::new(config);
    Router::new().merge(routes::payment::router()).with_state(state)
}

fn stripe_signature(secret: &str, timestamp: i64, payload: &[u8]) -> String {
    let mut signed_payload = timestamp.to_string().into_bytes();
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(payload);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac secret");
    mac.update(&signed_payload);
    let expected = mac.finalize().into_bytes();
    format!("t={},v1={}", timestamp, hex::encode(expected))
}

#[tokio::test]
async fn stripe_webhook_disabled_without_secret() {
    let mut config = Config::default();
    config.jwt_secret = "test-secret".to_string();
    config.stripe_webhook_secret = None;

    let response = app(config)
        .oneshot(
            Request::builder()
                .uri("/api/webhook/stripe")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"type":"checkout.session.completed","data":{"object":{}}}"#))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stripe_webhook_rejects_missing_signature_header() {
    let mut config = Config::default();
    config.jwt_secret = "test-secret".to_string();
    config.stripe_webhook_secret = Some("whsec_test".to_string());

    let response = app(config)
        .oneshot(
            Request::builder()
                .uri("/api/webhook/stripe")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"type":"checkout.session.completed","data":{"object":{"customer_email":"a@b.com"}}}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stripe_webhook_rejects_invalid_signature() {
    let mut config = Config::default();
    config.jwt_secret = "test-secret".to_string();
    config.stripe_webhook_secret = Some("whsec_test".to_string());

    let response = app(config)
        .oneshot(
            Request::builder()
                .uri("/api/webhook/stripe")
                .method("POST")
                .header("content-type", "application/json")
                .header("stripe-signature", "t=0,v1=deadbeef")
                .body(Body::from(
                    r#"{"type":"checkout.session.completed","data":{"object":{"customer_email":"a@b.com"}}}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stripe_webhook_accepts_valid_signature_and_issues_license() {
    let secret = "whsec_test";
    let mut config = Config::default();
    config.jwt_secret = "test-secret".to_string();
    config.stripe_webhook_secret = Some(secret.to_string());

    let payload =
        br#"{"type":"checkout.session.completed","data":{"object":{"customer_email":"a@b.com"}}}"#;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("now")
        .as_secs() as i64;
    let signature = stripe_signature(secret, timestamp, payload);

    let response = app(config.clone())
        .oneshot(
            Request::builder()
                .uri("/api/webhook/stripe")
                .method("POST")
                .header("content-type", "application/json")
                .header("stripe-signature", signature)
                .body(Body::from(payload.as_slice()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let parsed: serde_json::Value = serde_json::from_slice(&body).expect("json");
    let token = parsed
        .get("token")
        .and_then(|v| v.as_str())
        .expect("token string");
    assert!(parsed.get("pro").and_then(|v| v.as_bool()).unwrap_or(false));

    let claims = verify_license_token(token, &config.jwt_secret).expect("valid token");
    assert_eq!(claims.email, "a@b.com");
    assert!(claims.pro);
}

