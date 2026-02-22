use std::time::{Duration, Instant};

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::AppError,
    license::verify_license_token,
    pipeline::process,
    state::{AppState, StravaSession},
    types::activity::{AvailableData, Metrics, ParsedActivity, TrackPoint},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/strava/auth", post(strava_auth))
        .route("/api/strava/callback", get(strava_callback))
        .route("/api/strava/activities", get(list_activities))
        .route("/api/strava/activity/:activity_id", get(import_activity))
}

#[derive(Debug, Serialize)]
struct StravaAuthResponse {
    auth_url: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct StravaCallbackQuery {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize, Default)]
struct StravaAuthRequest {
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Debug, Serialize)]
struct StravaCallbackResponse {
    access_token: String,
    athlete_id: Option<u64>,
    expires_in_seconds: u64,
}

#[derive(Debug, Serialize)]
struct StravaActivitySummary {
    id: u64,
    name: String,
    distance_m: f64,
    start_date: Option<String>,
}

#[derive(Debug, Serialize)]
struct UploadLikeResponse {
    file_id: String,
    file_type: String,
    metrics: Metrics,
    available_data: AvailableData,
}

async fn strava_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<StravaAuthRequest>,
) -> Result<Json<StravaAuthResponse>, AppError> {
    require_pro_license(&state, &headers)?;

    let config = state.config();
    let provided_client_id = payload
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let provided_client_secret = payload
        .client_secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if provided_client_id.is_some() != provided_client_secret.is_some() {
        return Err(AppError::BadRequest(
            "Provide both Strava client_id and client_secret, or leave both empty".to_string(),
        ));
    }
    let client_id = if let Some(custom_client_id) = provided_client_id.as_ref() {
        custom_client_id.clone()
    } else {
        config
            .strava_client_id
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("STRAVA_CLIENT_ID is not configured".to_string()))?
            .to_string()
    };
    let redirect_uri = config.strava_redirect_uri.as_ref().ok_or_else(|| {
        AppError::BadRequest("STRAVA_REDIRECT_URI is not configured".to_string())
    })?;

    let oauth_state = Uuid::new_v4().to_string();
    state.store_strava_session(
        oauth_state.clone(),
        StravaSession {
            access_token: String::new(),
            athlete_id: None,
            expires_at: Instant::now() + Duration::from_secs(10 * 60),
            oauth_client_id: provided_client_id,
            oauth_client_secret: provided_client_secret,
        },
    );

    let auth_url = format!(
        "https://www.strava.com/oauth/authorize?client_id={}&response_type=code&redirect_uri={}&approval_prompt=auto&scope=read,activity:read_all&state={}",
        client_id, redirect_uri, oauth_state
    );

    Ok(Json(StravaAuthResponse {
        auth_url,
        state: oauth_state,
    }))
}

async fn strava_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<StravaCallbackQuery>,
) -> Result<Json<StravaCallbackResponse>, AppError> {
    require_pro_license(&state, &headers)?;
    let oauth_session = state
        .get_strava_session(&query.state)
        .ok_or_else(|| AppError::Unauthorized("Invalid OAuth state".to_string()))?;
    if !oauth_session.access_token.is_empty() {
        return Err(AppError::Unauthorized("Invalid OAuth state".to_string()));
    }

    let config = state.config();
    let client_id = oauth_session
        .oauth_client_id
        .as_deref()
        .or(config.strava_client_id.as_deref())
        .ok_or_else(|| AppError::BadRequest("STRAVA_CLIENT_ID is not configured".to_string()))?;
    let client_secret = oauth_session
        .oauth_client_secret
        .as_deref()
        .or(config.strava_client_secret.as_deref())
        .ok_or_else(|| AppError::BadRequest("STRAVA_CLIENT_SECRET is not configured".to_string()))?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://www.strava.com/oauth/token")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", query.code.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("Failed to exchange Strava OAuth token: {}", err)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "Strava token exchange failed ({}): {}",
            status, body
        )));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|err| AppError::Internal(format!("Invalid Strava response: {}", err)))?;
    let access_token = payload
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Internal("Strava response missing access_token".to_string()))?;
    let athlete_id = payload
        .get("athlete")
        .and_then(|athlete| athlete.get("id"))
        .and_then(Value::as_u64);
    let expires_at_unix = payload.get("expires_at").and_then(Value::as_i64).unwrap_or(0);
    let expires_in_seconds = payload
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(6 * 3600);

    let expires_at = if expires_at_unix > 0 {
        let now = Utc::now().timestamp();
        let delta = (expires_at_unix - now).max(30) as u64;
        Instant::now() + Duration::from_secs(delta)
    } else {
        Instant::now() + Duration::from_secs(expires_in_seconds)
    };

    state.store_strava_session(
        access_token.to_string(),
        StravaSession {
            access_token: access_token.to_string(),
            athlete_id,
            expires_at,
            oauth_client_id: None,
            oauth_client_secret: None,
        },
    );

    Ok(Json(StravaCallbackResponse {
        access_token: access_token.to_string(),
        athlete_id,
        expires_in_seconds,
    }))
}

#[derive(Deserialize)]
struct ListActivitiesQuery {
    page: Option<u32>,
}

async fn list_activities(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListActivitiesQuery>,
) -> Result<Json<Vec<StravaActivitySummary>>, AppError> {
    let access_token = bearer_token(&headers)
        .ok_or_else(|| AppError::Unauthorized("Missing Strava Bearer token".to_string()))?;
    let session = state
        .get_strava_session(&access_token)
        .ok_or_else(|| AppError::Unauthorized("Expired or unknown Strava session".to_string()))?;

    let page = params.page.unwrap_or(1);
    let url = format!(
        "https://www.strava.com/api/v3/athlete/activities?per_page=100&page={}",
        page
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .bearer_auth(&session.access_token)
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("Failed to fetch Strava activities: {}", err)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "Strava activities request failed ({}): {}",
            status, body
        )));
    }

    let payload: Vec<Value> = response
        .json()
        .await
        .map_err(|err| AppError::Internal(format!("Invalid Strava activities response: {}", err)))?;

    let activities = payload
        .into_iter()
        .filter_map(|activity| {
            let id = activity.get("id").and_then(Value::as_u64)?;
            Some(StravaActivitySummary {
                id,
                name: activity
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("Activity")
                    .to_string(),
                distance_m: activity
                    .get("distance")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                start_date: activity
                    .get("start_date")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string()),
            })
        })
        .collect();

    Ok(Json(activities))
}

async fn import_activity(
    State(state): State<AppState>,
    Path(activity_id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<UploadLikeResponse>, AppError> {
    let access_token = bearer_token(&headers)
        .ok_or_else(|| AppError::Unauthorized("Missing Strava Bearer token".to_string()))?;
    let session = state
        .get_strava_session(&access_token)
        .ok_or_else(|| AppError::Unauthorized("Expired or unknown Strava session".to_string()))?;

    let client = reqwest::Client::new();
    let streams_url = format!(
        "https://www.strava.com/api/v3/activities/{}/streams?keys=latlng,altitude,time,heartrate,watts&key_by_type=true",
        activity_id
    );
    let response = client
        .get(streams_url)
        .bearer_auth(&session.access_token)
        .send()
        .await
        .map_err(|err| AppError::Internal(format!("Failed to fetch Strava activity streams: {}", err)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "Strava streams request failed ({}): {}",
            status, body
        )));
    }

    let streams: Value = response
        .json()
        .await
        .map_err(|err| AppError::Internal(format!("Invalid Strava streams response: {}", err)))?;

    let latlng_data = streams
        .get("latlng")
        .and_then(|entry| entry.get("data"))
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::BadRequest("Strava stream missing latlng data".to_string()))?;

    let altitude_data = streams
        .get("altitude")
        .and_then(|entry| entry.get("data"))
        .and_then(Value::as_array);
    let time_data = streams
        .get("time")
        .and_then(|entry| entry.get("data"))
        .and_then(Value::as_array);
    let hr_data = streams
        .get("heartrate")
        .and_then(|entry| entry.get("data"))
        .and_then(Value::as_array);
    let watts_data = streams
        .get("watts")
        .and_then(|entry| entry.get("data"))
        .and_then(Value::as_array);

    let now = Utc::now();
    let mut points = Vec::with_capacity(latlng_data.len());
    for (idx, pair) in latlng_data.iter().enumerate() {
        let coord = pair.as_array().ok_or_else(|| {
            AppError::BadRequest("Unexpected Strava latlng format".to_string())
        })?;
        if coord.len() < 2 {
            continue;
        }

        let lat = coord.first().and_then(Value::as_f64).unwrap_or(0.0);
        let lon = coord.get(1).and_then(Value::as_f64).unwrap_or(0.0);
        let elevation = altitude_data
            .and_then(|arr| arr.get(idx))
            .and_then(Value::as_f64);
        let elapsed_seconds = time_data
            .and_then(|arr| arr.get(idx))
            .and_then(Value::as_i64)
            .unwrap_or(idx as i64);
        let time = Utc
            .timestamp_opt(now.timestamp() + elapsed_seconds, 0)
            .single();
        let heart_rate = hr_data
            .and_then(|arr| arr.get(idx))
            .and_then(Value::as_u64)
            .map(|v| v as u16);
        let power = watts_data
            .and_then(|arr| arr.get(idx))
            .and_then(Value::as_u64)
            .map(|v| v as u16);

        points.push(TrackPoint {
            lat,
            lon,
            elevation,
            time,
            heart_rate,
            power,
            cadence: None,
            temperature: None,
        });
    }

    let parsed = ParsedActivity { points };
    let processed = process::process(&parsed)?;
    let file_id = Uuid::new_v4().to_string();
    state.insert(file_id.clone(), processed.clone());

    Ok(Json(UploadLikeResponse {
        file_id,
        file_type: "strava".to_string(),
        metrics: processed.metrics,
        available_data: processed.available_data,
    }))
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("authorization")?;
    let raw = value.to_str().ok()?;
    raw.strip_prefix("Bearer ").map(|token| token.trim().to_string())
}

fn require_pro_license(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let token = bearer_token(headers)
        .ok_or_else(|| AppError::Unauthorized("Missing license bearer token".to_string()))?;
    let claims = verify_license_token(&token, &state.config().jwt_secret)
        .map_err(|_| AppError::Unauthorized("Invalid license token".to_string()))?;
    if !claims.pro {
        return Err(AppError::Unauthorized(
            "Pro license required for Strava import".to_string(),
        ));
    }
    Ok(())
}
