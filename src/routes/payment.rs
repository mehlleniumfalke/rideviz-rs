use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::AppError,
    license::{create_license_token, verify_license_token},
    state::{AppState, CachedLicense},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/checkout", post(create_checkout))
        .route("/api/checkout/complete", post(complete_checkout))
        .route("/api/webhook/stripe", post(stripe_webhook))
        .route("/api/license/verify", get(verify_license))
}

#[derive(Debug, Deserialize)]
struct CheckoutRequest {
    email: String,
    success_url: Option<String>,
    cancel_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct CheckoutResponse {
    checkout_url: String,
    mode: &'static str,
}

#[derive(Debug, Deserialize)]
struct StripeWebhookPayload {
    #[serde(rename = "type")]
    event_type: String,
    data: StripeWebhookData,
}

#[derive(Debug, Deserialize)]
struct StripeWebhookData {
    object: Value,
}

#[derive(Debug, Deserialize)]
struct CheckoutCompleteRequest {
    session_id: String,
}

#[derive(Debug, Serialize)]
struct LicenseResponse {
    token: String,
    pro: bool,
    expires_in_seconds: u64,
}

#[derive(Debug, Serialize)]
struct VerifyLicenseResponse {
    valid: bool,
    pro: bool,
    email: String,
}

async fn create_checkout(
    State(state): State<AppState>,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<CheckoutResponse>, AppError> {
    if req.email.trim().is_empty() {
        return Err(AppError::BadRequest("Email is required".to_string()));
    }

    let config = state.config();
    let success_url = req
        .success_url
        .unwrap_or_else(|| format!("{}/app?checkout=success", config.app_base_url));
    let cancel_url = req
        .cancel_url
        .unwrap_or_else(|| format!("{}/app?checkout=cancel", config.app_base_url));

    let Some(secret) = &config.stripe_secret_key else {
        return Ok(Json(CheckoutResponse {
            checkout_url: format!(
                "{}/app?checkout=mock&email={}",
                config.app_base_url,
                req.email
            ),
            mode: "mock",
        }));
    };

    let Some(price_id) = &config.stripe_price_id else {
        return Err(AppError::BadRequest(
            "STRIPE_PRICE_ID is not configured".to_string(),
        ));
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .bearer_auth(secret)
        .form(&[
            ("mode", "payment"),
            ("success_url", success_url.as_str()),
            ("cancel_url", cancel_url.as_str()),
            ("customer_email", req.email.as_str()),
            ("line_items[0][price]", price_id.as_str()),
            ("line_items[0][quantity]", "1"),
        ])
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("Failed to create Stripe checkout session: {}", err)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Stripe checkout request failed ({}): {}",
            status, body
        )));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|err| AppError::Internal(format!("Invalid Stripe response: {}", err)))?;
    let checkout_url = payload
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Internal("Stripe response missing checkout URL".to_string()))?;

    Ok(Json(CheckoutResponse {
        checkout_url: checkout_url.to_string(),
        mode: "live",
    }))
}

async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<StripeWebhookPayload>,
) -> Result<Json<LicenseResponse>, AppError> {
    if state.config().stripe_webhook_secret.is_some()
        && headers.get("stripe-signature").is_none()
    {
        return Err(AppError::Unauthorized(
            "Missing Stripe signature header".to_string(),
        ));
    }

    let completed = payload.event_type == "checkout.session.completed";
    if !completed {
        return Err(AppError::BadRequest(format!(
            "Unhandled webhook event type: {}",
            payload.event_type
        )));
    }

    let email = payload
        .data
        .object
        .get("customer_email")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .data
                .object
                .get("customer_details")
                .and_then(|details| details.get("email"))
                .and_then(Value::as_str)
        })
        .ok_or_else(|| AppError::BadRequest("Stripe webhook missing customer email".to_string()))?;

    Ok(Json(issue_license_for_email(&state, email)?))
}

async fn complete_checkout(
    State(state): State<AppState>,
    Json(req): Json<CheckoutCompleteRequest>,
) -> Result<Json<LicenseResponse>, AppError> {
    if req.session_id.trim().is_empty() {
        return Err(AppError::BadRequest("session_id is required".to_string()));
    }

    let Some(secret) = &state.config().stripe_secret_key else {
        return Err(AppError::BadRequest(
            "STRIPE_SECRET_KEY is not configured".to_string(),
        ));
    };

    let session_url = format!(
        "https://api.stripe.com/v1/checkout/sessions/{}",
        req.session_id.trim()
    );
    let client = reqwest::Client::new();
    let response = client
        .get(session_url)
        .bearer_auth(secret)
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("Failed to fetch Stripe checkout session: {}", err)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "Stripe checkout session lookup failed ({}): {}",
            status, body
        )));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|err| AppError::Internal(format!("Invalid Stripe session response: {}", err)))?;
    let payment_status = payload
        .get("payment_status")
        .and_then(Value::as_str)
        .unwrap_or("");
    let status = payload.get("status").and_then(Value::as_str).unwrap_or("");
    if payment_status != "paid" && status != "complete" {
        return Err(AppError::BadRequest(
            "Checkout session is not paid yet".to_string(),
        ));
    }

    let email = payload
        .get("customer_email")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("customer_details")
                .and_then(|details| details.get("email"))
                .and_then(Value::as_str)
        })
        .ok_or_else(|| AppError::BadRequest("Stripe session missing customer email".to_string()))?;

    Ok(Json(issue_license_for_email(&state, email)?))
}

async fn verify_license(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<VerifyLicenseResponse>, AppError> {
    let token = bearer_token(&headers)
        .ok_or_else(|| AppError::Unauthorized("Missing Bearer token".to_string()))?;

    let claims = verify_license_token(&token, &state.config().jwt_secret)?;
    let in_cache = state.verify_license(&token);
    let is_pro = in_cache
        .as_ref()
        .map(|entry| entry.is_pro)
        .unwrap_or(claims.pro);
    let email = in_cache
        .as_ref()
        .map(|entry| entry.email.clone())
        .unwrap_or(claims.email);

    Ok(Json(VerifyLicenseResponse {
        valid: true,
        pro: is_pro,
        email,
    }))
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("authorization")?;
    let raw = value.to_str().ok()?;
    raw.strip_prefix("Bearer ").map(|token| token.trim().to_string())
}

fn issue_license_for_email(state: &AppState, email: &str) -> Result<LicenseResponse, AppError> {
    const LIFETIME_SECONDS: u64 = 100 * 365 * 24 * 3600;

    let token = create_license_token(
        &Uuid::new_v4().to_string(),
        email,
        true,
        LIFETIME_SECONDS,
        &state.config().jwt_secret,
    )?;

    state.store_license(CachedLicense {
        token: token.clone(),
        email: email.to_string(),
        is_pro: true,
        expires_at: Instant::now() + Duration::from_secs(LIFETIME_SECONDS),
    });

    Ok(LicenseResponse {
        token,
        pro: true,
        expires_in_seconds: LIFETIME_SECONDS,
    })
}
