use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path as FsPath, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::license::verify_license_token;
use crate::pipeline::{prepare, progress, rasterize, render};
use crate::state::AppState;
use crate::types::{
    activity::{AvailableData, Metrics},
    gradient::Gradient,
    viz::{ColorByMetric, OutputConfig, RenderOptions, RoutePoint, StatOverlayItem, VizData},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/visualize", post(visualize))
        .route("/api/export/video", post(export_video))
        .route("/api/route-data/:file_id", get(route_data))
}

#[derive(Serialize)]
struct ExportVideoErrorBody {
    error: String,
    code: &'static str,
    request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
}

fn export_video_error_response(
    status: StatusCode,
    code: &'static str,
    request_id: &str,
    message: String,
    retry_after_seconds: Option<u64>,
) -> Response {
    let mut response = (
        status,
        Json(ExportVideoErrorBody {
            error: message,
            code,
            request_id: request_id.to_string(),
            retry_after_seconds,
        }),
    )
        .into_response();

    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap_or_else(|_| "invalid".parse().unwrap()),
    );

    if let Some(seconds) = retry_after_seconds {
        response.headers_mut().insert(
            header::RETRY_AFTER,
            seconds
                .to_string()
                .parse()
                .unwrap_or_else(|_| "1".parse().unwrap()),
        );
    }

    response
}

fn app_error_status_code(err: &AppError) -> StatusCode {
    match err {
        AppError::Parse(_)
        | AppError::Process(_)
        | AppError::Prepare(_)
        | AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        AppError::Internal(_) | AppError::Render(_) | AppError::Raster(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

fn app_error_code(err: &AppError) -> &'static str {
    match err {
        AppError::Parse(_)
        | AppError::Process(_)
        | AppError::Prepare(_)
        | AppError::BadRequest(_) => "bad_request",
        AppError::NotFound(_) => "not_found",
        AppError::Unauthorized(_) => "unauthorized",
        AppError::Internal(_) | AppError::Render(_) | AppError::Raster(_) => "internal",
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VisualizeRequest {
    file_id: String,
    #[serde(default = "default_gradient")]
    gradient: String,
    width: Option<u32>,
    height: Option<u32>,
    color_by: Option<String>,
    #[serde(default = "default_stroke_width")]
    stroke_width: f32,
    #[serde(default = "default_padding")]
    padding: u32,
    #[serde(default = "default_smoothing")]
    smoothing: usize,
    #[serde(default = "default_true")]
    glow: bool,
    background: Option<String>,
    #[serde(default)]
    stats: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VideoExportRequest {
    file_id: String,
    #[serde(default = "default_gradient")]
    gradient: String,
    width: Option<u32>,
    height: Option<u32>,
    color_by: Option<String>,
    #[serde(default = "default_stroke_width")]
    stroke_width: f32,
    #[serde(default = "default_padding")]
    padding: u32,
    #[serde(default = "default_smoothing")]
    smoothing: usize,
    #[serde(default = "default_true")]
    glow: bool,
    background: Option<String>,
    duration_seconds: f32,
    fps: u32,
    #[serde(default)]
    stats: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RouteDataQuery {
    color_by: Option<String>,
    #[serde(default = "default_smoothing")]
    smoothing: usize,
}

#[derive(Serialize)]
struct RouteDataResponse {
    file_id: String,
    viz_data: VizData,
    metrics: Metrics,
    available_data: AvailableData,
}

fn default_gradient() -> String {
    "fire".to_string()
}

fn default_stroke_width() -> f32 {
    3.0
}

fn default_padding() -> u32 {
    40
}

fn default_smoothing() -> usize {
    30
}

fn default_true() -> bool {
    true
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), AppError> {
    const MIN_DIM: u32 = 320;
    const MAX_DIM: u32 = 4096;
    const MAX_MEGAPIXELS: f64 = 10.0;

    if !(MIN_DIM..=MAX_DIM).contains(&width) || !(MIN_DIM..=MAX_DIM).contains(&height) {
        return Err(AppError::BadRequest(format!(
            "Invalid dimensions: {}x{}. Width/height must be between {} and {}",
            width, height, MIN_DIM, MAX_DIM
        )));
    }

    let megapixels = (width as f64 * height as f64) / 1_000_000.0;
    if megapixels > MAX_MEGAPIXELS {
        return Err(AppError::BadRequest(format!(
            "Image too large: {}x{} ({:.2} MP). Max allowed is {:.1} MP",
            width, height, megapixels, MAX_MEGAPIXELS
        )));
    }

    Ok(())
}

fn cap_mp4_dimensions_to_720p(width: u32, height: u32) -> (u32, u32) {
    const MAX_PIXELS_720P: f64 = 1280.0 * 720.0;
    let pixels = width as f64 * height as f64;
    if pixels <= MAX_PIXELS_720P {
        // Keep encoder-friendly even dimensions.
        return (width & !1, height & !1);
    }

    let scale = (MAX_PIXELS_720P / pixels).sqrt();
    let mut scaled_width = ((width as f64) * scale).round() as u32;
    let mut scaled_height = ((height as f64) * scale).round() as u32;
    if scaled_width % 2 != 0 {
        scaled_width = scaled_width.saturating_sub(1);
    }
    if scaled_height % 2 != 0 {
        scaled_height = scaled_height.saturating_sub(1);
    }
    (scaled_width.max(320), scaled_height.max(320))
}

/// Maps smoothing level (0-100) to internal route rendering parameters.
/// Returns (simplify stride, curve tension).
fn smoothing_to_route_params(level: usize) -> (usize, f32) {
    let t = level.min(100) as f32 / 100.0;
    let simplify = (1.0 + t * 29.0).round() as usize; // 1 -> 30
    let tension = t * 0.45; // 0.0 -> 0.45
    (simplify, tension)
}

fn format_duration(duration_seconds: u64) -> String {
    let hours = duration_seconds / 3600;
    let minutes = (duration_seconds % 3600) / 60;
    let seconds = duration_seconds % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[derive(Debug, Clone)]
struct StatOverlaySpec {
    key: String,
    label: String,
    color_t: f64,
}

fn stat_key_to_label(
    key: &str,
    metrics: &Metrics,
    available_data: &AvailableData,
) -> Option<String> {
    match key {
        "distance" => Some("DIST".to_string()),
        "duration" if metrics.duration_seconds > 0 => Some("DUR".to_string()),
        "elevation_gain" if available_data.has_elevation => Some("GAIN".to_string()),
        "avg_speed" if metrics.duration_seconds > 0 => Some("AVG SPD".to_string()),
        "avg_heart_rate" if available_data.has_heart_rate && metrics.avg_heart_rate.is_some() => {
            Some("AVG HR".to_string())
        }
        "max_heart_rate" if available_data.has_heart_rate && metrics.max_heart_rate.is_some() => {
            Some("MAX HR".to_string())
        }
        "avg_power" if available_data.has_power && metrics.avg_power.is_some() => {
            Some("AVG PWR".to_string())
        }
        "max_power" if available_data.has_power && metrics.max_power.is_some() => {
            Some("MAX PWR".to_string())
        }
        _ => None,
    }
}

fn build_stats_overlay_specs(
    requested_keys: Option<&Vec<String>>,
    metrics: &Metrics,
    available_data: &AvailableData,
) -> Result<Vec<StatOverlaySpec>, AppError> {
    let Some(keys) = requested_keys else {
        return Ok(Vec::new());
    };
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let allowed: HashSet<&str> = [
        "distance",
        "duration",
        "elevation_gain",
        "avg_speed",
        "avg_heart_rate",
        "max_heart_rate",
        "avg_power",
        "max_power",
    ]
    .into_iter()
    .collect();

    for key in keys {
        if !allowed.contains(key.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid stat key: {}. Allowed: distance, duration, elevation_gain, avg_speed, avg_heart_rate, max_heart_rate, avg_power, max_power",
                key
            )));
        }
    }

    let mut seen = HashSet::new();
    let mut specs: Vec<(String, String)> = Vec::new();
    for key in keys {
        if !seen.insert(key.to_string()) {
            continue;
        }
        if let Some(label) = stat_key_to_label(key, metrics, available_data) {
            specs.push((key.clone(), label));
        }
    }

    let item_count = specs.len().max(1) as f64;
    Ok(specs
        .into_iter()
        .enumerate()
        .map(|(idx, (key, label))| {
            let t = if item_count <= 1.0 {
                0.5
            } else {
                idx as f64 / (item_count - 1.0)
            };
            StatOverlaySpec {
                key,
                label,
                color_t: t,
            }
        })
        .collect())
}

#[derive(Debug, Clone, Copy)]
struct RouteTelemetrySample {
    distance_km: f64,
    elevation_gain_m: f64,
    elapsed_seconds: Option<f64>,
    avg_heart_rate: Option<f64>,
    max_heart_rate: Option<f64>,
    avg_power: Option<f64>,
    max_power: Option<f64>,
}

fn telemetry_from_point(point: &RoutePoint) -> RouteTelemetrySample {
    RouteTelemetrySample {
        distance_km: point.cumulative_distance_km,
        elevation_gain_m: point.cumulative_elevation_gain_m,
        elapsed_seconds: point.elapsed_seconds,
        avg_heart_rate: point.cumulative_avg_heart_rate,
        max_heart_rate: point.cumulative_max_heart_rate,
        avg_power: point.cumulative_avg_power,
        max_power: point.cumulative_max_power,
    }
}

fn interpolate_optional(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x + (y - x) * t),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

fn sample_route_telemetry(data: &VizData, progress: f64) -> Option<RouteTelemetrySample> {
    if data.points.is_empty() {
        return None;
    }
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return data.points.first().map(telemetry_from_point);
    }
    if progress >= 1.0 {
        return data.points.last().map(telemetry_from_point);
    }

    for idx in 0..data.points.len().saturating_sub(1) {
        let current = &data.points[idx];
        let next = &data.points[idx + 1];
        if next.route_progress <= current.route_progress {
            continue;
        }
        if next.route_progress < progress {
            continue;
        }
        let local_t = ((progress - current.route_progress)
            / (next.route_progress - current.route_progress))
            .clamp(0.0, 1.0);
        return Some(RouteTelemetrySample {
            distance_km: current.cumulative_distance_km
                + (next.cumulative_distance_km - current.cumulative_distance_km) * local_t,
            elevation_gain_m: current.cumulative_elevation_gain_m
                + (next.cumulative_elevation_gain_m - current.cumulative_elevation_gain_m)
                    * local_t,
            elapsed_seconds: interpolate_optional(
                current.elapsed_seconds,
                next.elapsed_seconds,
                local_t,
            ),
            avg_heart_rate: interpolate_optional(
                current.cumulative_avg_heart_rate,
                next.cumulative_avg_heart_rate,
                local_t,
            ),
            max_heart_rate: interpolate_optional(
                current.cumulative_max_heart_rate,
                next.cumulative_max_heart_rate,
                local_t,
            ),
            avg_power: interpolate_optional(
                current.cumulative_avg_power,
                next.cumulative_avg_power,
                local_t,
            ),
            max_power: interpolate_optional(
                current.cumulative_max_power,
                next.cumulative_max_power,
                local_t,
            ),
        });
    }

    data.points.last().map(telemetry_from_point)
}

fn fallback_telemetry(metrics: &Metrics) -> RouteTelemetrySample {
    RouteTelemetrySample {
        distance_km: metrics.distance_km,
        elevation_gain_m: metrics.elevation_gain_m,
        elapsed_seconds: if metrics.duration_seconds > 0 {
            Some(metrics.duration_seconds as f64)
        } else {
            None
        },
        avg_heart_rate: metrics.avg_heart_rate.map(|v| v as f64),
        max_heart_rate: metrics.max_heart_rate.map(|v| v as f64),
        avg_power: metrics.avg_power.map(|v| v as f64),
        max_power: metrics.max_power.map(|v| v as f64),
    }
}

fn stat_value_for_progress(
    key: &str,
    metrics: &Metrics,
    telemetry: &RouteTelemetrySample,
) -> Option<String> {
    match key {
        "distance" => Some(format!("{:.1} km", telemetry.distance_km)),
        "duration" if metrics.duration_seconds > 0 => telemetry
            .elapsed_seconds
            .map(|seconds| format_duration(seconds.max(0.0).round() as u64)),
        "elevation_gain" => Some(format!("{:.0} m", telemetry.elevation_gain_m.max(0.0))),
        "avg_speed" if metrics.duration_seconds > 0 => telemetry.elapsed_seconds.map(|elapsed| {
            let speed = if elapsed > f64::EPSILON {
                (telemetry.distance_km / elapsed) * 3600.0
            } else {
                0.0
            };
            format!("{:.1} km/h", speed.max(0.0))
        }),
        "avg_heart_rate" => telemetry
            .avg_heart_rate
            .map(|value| format!("{:.0} bpm", value.max(0.0))),
        "max_heart_rate" => telemetry
            .max_heart_rate
            .map(|value| format!("{:.0} bpm", value.max(0.0))),
        "avg_power" => telemetry
            .avg_power
            .map(|value| format!("{:.0} W", value.max(0.0))),
        "max_power" => telemetry
            .max_power
            .map(|value| format!("{:.0} W", value.max(0.0))),
        _ => None,
    }
}

fn build_stats_overlay_items_at_progress(
    specs: &[StatOverlaySpec],
    data: &VizData,
    metrics: &Metrics,
    progress: f64,
) -> Vec<StatOverlayItem> {
    if specs.is_empty() {
        return Vec::new();
    }

    let telemetry = sample_route_telemetry(data, progress).unwrap_or_else(|| fallback_telemetry(metrics));
    specs
        .iter()
        .map(|spec| StatOverlayItem {
            label: spec.label.clone(),
            value: stat_value_for_progress(&spec.key, metrics, &telemetry)
                .unwrap_or_else(|| "-".to_string()),
            color_t: spec.color_t,
        })
        .collect()
}

async fn visualize(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<VisualizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let processed = state
        .get(&req.file_id)
        .ok_or_else(|| AppError::NotFound(req.file_id.clone()))?;

    let mut options = RenderOptions::route_3d_defaults();
    options.gradient = Gradient::get(&req.gradient).unwrap_or_else(Gradient::default);
    match (req.width, req.height) {
        (Some(width), Some(height)) => {
            validate_dimensions(width, height)?;
            options.width = width;
            options.height = height;
        }
        (None, None) => {}
        _ => {
            return Err(AppError::BadRequest(
                "Both width and height must be provided together".to_string(),
            ))
        }
    }
    options.stroke_width = req.stroke_width;
    options.padding = req.padding;
    options.smoothing = req.smoothing;
    options.glow = req.glow;
    options.color_by = match req.color_by.as_deref() {
        Some(metric) => Some(ColorByMetric::from_str(metric).ok_or_else(|| {
            AppError::BadRequest(format!(
                "Invalid color_by: {}. Use 'elevation', 'speed', 'heartrate', or 'power'",
                metric
            ))
        })?),
        None => None,
    };

    let (simplify, curve_tension) = smoothing_to_route_params(req.smoothing);
    options.simplify = simplify;
    options.curve_tension = curve_tension;

    let viz_data = prepare::prepare(&processed, &options)?;
    let stats_specs = build_stats_overlay_specs(
        req.stats.as_ref(),
        &processed.metrics,
        &processed.available_data,
    )?;
    
    let background = match req.background.as_deref() {
        Some("white") => Some((255, 255, 255, 255)),
        Some("black") => Some((0, 0, 0, 255)),
        Some("transparent") | None => None,
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "Invalid background: {}. Use 'transparent', 'white', or 'black'",
                other
            )));
        }
    };
    
    let pro_license = bearer_token(&headers)
        .and_then(|token| verify_license_token(&token, &state.config().jwt_secret).ok())
        .map(|claims| claims.pro)
        .unwrap_or(false);

    let output_config = OutputConfig {
        width: options.width,
        height: options.height,
        background,
        watermark: !pro_license,
    };

    let viz_data_for_render = viz_data.clone();
    let metrics_for_render = processed.metrics.clone();
    let options_for_render = options.clone();
    let output_for_render = output_config.clone();
    let specs_for_render = stats_specs.clone();
    let image_bytes = tokio::task::spawn_blocking(move || {
        // Static image - render single frame at progress=1.0 (full route)
        let stats_for_frame = build_stats_overlay_items_at_progress(
            &specs_for_render,
            &viz_data_for_render,
            &metrics_for_render,
            1.0,
        );
        let svg = render::render_svg_frame(
            &viz_data_for_render,
            &options_for_render,
            1.0,
            &stats_for_frame,
        )
        .map_err(|err| {
            crate::error::RasterError::RenderFailed(format!(
                "Failed to render static frame: {}",
                err
            ))
        })?;
        rasterize::rasterize(&svg, &output_for_render)
    })
    .await
    .map_err(|err| AppError::Internal(format!("Rendering task join failed: {}", err)))??;
    
    tracing::info!("Generated PNG: {} bytes", image_bytes.len());

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        image_bytes,
    ))
}

async fn export_video(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<VideoExportRequest>,
) -> Response {
    const MAX_MP4_DURATION_SECONDS: f32 = 15.0;
    const MAX_MP4_FPS: u32 = 30;
    const MAX_MP4_FRAMES: u32 = 450;

    let request_id = Uuid::new_v4().to_string();
    let t0 = Instant::now();

    let token = match bearer_token(&headers) {
        Some(token) => token,
        None => {
            return export_video_error_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                &request_id,
                "Missing bearer token".to_string(),
                None,
            );
        }
    };

    let claims = match verify_license_token(&token, &state.config().jwt_secret) {
        Ok(claims) => claims,
        Err(_) => {
            return export_video_error_response(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                &request_id,
                "Invalid license token".to_string(),
                None,
            );
        }
    };

    if !claims.pro {
        return export_video_error_response(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            &request_id,
            "Pro license required for MP4 export".to_string(),
            None,
        );
    }

    let rate_limit_key = claims.sub;
    if let Err(retry_after_seconds) = state.video_export_rate_limiter().check(&rate_limit_key) {
        tracing::warn!(
            request_id = %request_id,
            retry_after_seconds,
            "MP4 export rate-limited"
        );
        return export_video_error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            &request_id,
            format!(
                "Too many MP4 export requests. Try again in {}s.",
                retry_after_seconds
            ),
            Some(retry_after_seconds),
        );
    }

    let semaphore = state.video_export_semaphore();
    let queue_timeout = state.config().video_export_queue_timeout;
    let permit = match tokio::time::timeout(queue_timeout, semaphore.acquire_owned()).await {
        Ok(Ok(permit)) => permit,
        Ok(Err(_)) => {
            return export_video_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "export_busy",
                &request_id,
                "MP4 export service is unavailable".to_string(),
                Some(1),
            );
        }
        Err(_) => {
            let retry_after_seconds = queue_timeout.as_secs().max(1);
            tracing::warn!(request_id = %request_id, "MP4 export concurrency limit reached");
            return export_video_error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "export_busy",
                &request_id,
                "MP4 export capacity is busy. Try again shortly.".to_string(),
                Some(retry_after_seconds),
            );
        }
    };

    let mut options = RenderOptions::route_3d_defaults();
    options.gradient = Gradient::get(&req.gradient).unwrap_or_else(Gradient::default);
    match (req.width, req.height) {
        (Some(width), Some(height)) => {
            if let Err(err) = validate_dimensions(width, height) {
                return export_video_error_response(
                    app_error_status_code(&err),
                    app_error_code(&err),
                    &request_id,
                    err.to_string(),
                    None,
                );
            }
            options.width = width;
            options.height = height;
        }
        (None, None) => {}
        _ => {
            let err = AppError::BadRequest(
                "Both width and height must be provided together".to_string(),
            );
            return export_video_error_response(
                app_error_status_code(&err),
                app_error_code(&err),
                &request_id,
                err.to_string(),
                None,
            );
        }
    }

    let (video_width, video_height) = cap_mp4_dimensions_to_720p(options.width, options.height);
    if video_width != options.width || video_height != options.height {
        tracing::info!(
            "Capped MP4 dimensions from {}x{} to {}x{}",
            options.width,
            options.height,
            video_width,
            video_height
        );
    }
    options.width = video_width;
    options.height = video_height;

    options.stroke_width = req.stroke_width;
    options.padding = req.padding;
    options.smoothing = req.smoothing;
    options.glow = req.glow;
    options.color_by = match req.color_by.as_deref() {
        Some(metric) => match ColorByMetric::from_str(metric) {
            Some(metric) => Some(metric),
            None => {
                let err = AppError::BadRequest(format!(
                    "Invalid color_by: {}. Use 'elevation', 'speed', 'heartrate', or 'power'",
                    metric
                ));
                return export_video_error_response(
                    app_error_status_code(&err),
                    app_error_code(&err),
                    &request_id,
                    err.to_string(),
                    None,
                );
            }
        },
        None => None,
    };

    let fps = req.fps.clamp(24, MAX_MP4_FPS);
    let duration_seconds = req.duration_seconds.clamp(3.0, MAX_MP4_DURATION_SECONDS);
    let requested_frame_count = (duration_seconds * fps as f32).round() as u32;
    let frame_count = requested_frame_count.clamp(24, MAX_MP4_FRAMES);
    options.animation_frames = frame_count;
    options.animation_duration_ms = ((frame_count as f32 / fps as f32) * 1000.0).round() as u32;

    let (simplify, curve_tension) = smoothing_to_route_params(req.smoothing);
    options.simplify = simplify;
    options.curve_tension = curve_tension;

    let background = match req.background.as_deref() {
        Some("white") | None => Some((255, 255, 255, 255)),
        Some("black") => Some((0, 0, 0, 255)),
        Some("transparent") => {
            let err = AppError::BadRequest(
                "MP4 export does not support transparent background".to_string(),
            );
            return export_video_error_response(
                app_error_status_code(&err),
                app_error_code(&err),
                &request_id,
                err.to_string(),
                None,
            );
        }
        Some(other) => {
            let err = AppError::BadRequest(format!(
                "Invalid background: {}. Use 'white' or 'black'",
                other
            ));
            return export_video_error_response(
                app_error_status_code(&err),
                app_error_code(&err),
                &request_id,
                err.to_string(),
                None,
            );
        }
    };

    let output_config = OutputConfig {
        width: options.width,
        height: options.height,
        background,
        watermark: false,
    };

    let processed = match state.get(&req.file_id) {
        Some(processed) => processed,
        None => {
            let err = AppError::NotFound(req.file_id.clone());
            return export_video_error_response(
                app_error_status_code(&err),
                app_error_code(&err),
                &request_id,
                err.to_string(),
                None,
            );
        }
    };

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_for_task = cancel.clone();
    let request_id_for_log = request_id.clone();
    let stats_requested = req.stats.clone();

    let mut handle = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        if cancel_for_task.load(Ordering::Relaxed) {
            return Err(AppError::Internal("MP4 export cancelled".to_string()));
        }

        let viz_data = prepare::prepare(&processed, &options)?;
        let stats_specs = build_stats_overlay_specs(
            stats_requested.as_ref(),
            &processed.metrics,
            &processed.available_data,
        )?;

        render_mp4_video(
            &viz_data,
            &options,
            &output_config,
            &stats_specs,
            &processed.metrics,
            fps,
            cancel_for_task.as_ref(),
        )
    });

    let render_timeout = state.config().video_export_timeout;
    let video_bytes = match tokio::select! {
        joined = &mut handle => Ok(joined),
        _ = tokio::time::sleep(render_timeout) => Err(()),
    } {
        Ok(joined) => match joined {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(err)) => {
                tracing::error!(request_id = %request_id_for_log, "MP4 export failed: {}", err);
                return export_video_error_response(
                    app_error_status_code(&err),
                    app_error_code(&err),
                    &request_id,
                    err.to_string(),
                    None,
                );
            }
            Err(err) => {
                let app_err =
                    AppError::Internal(format!("Video export task join failed: {}", err));
                tracing::error!(request_id = %request_id_for_log, "MP4 export join failed: {}", app_err);
                return export_video_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    &request_id,
                    app_err.to_string(),
                    None,
                );
            }
        },
        Err(_) => {
            cancel.store(true, Ordering::Relaxed);
            handle.abort();
            tracing::warn!(
                request_id = %request_id,
                timeout_seconds = render_timeout.as_secs(),
                "MP4 export timed out"
            );
            return export_video_error_response(
                StatusCode::GATEWAY_TIMEOUT,
                "export_timeout",
                &request_id,
                format!(
                    "MP4 export timed out after {}s. Try a smaller size or shorter duration.",
                    render_timeout.as_secs()
                ),
                None,
            );
        }
    };

    tracing::info!(
        request_id = %request_id,
        bytes = video_bytes.len(),
        elapsed_ms = t0.elapsed().as_millis(),
        "Generated MP4"
    );

    let mut response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "video/mp4"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rideviz-route.mp4\"",
            ),
        ],
        video_bytes,
    )
        .into_response();
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap_or_else(|_| "invalid".parse().unwrap()),
    );
    response
}

fn render_mp4_video(
    data: &VizData,
    options: &RenderOptions,
    output: &OutputConfig,
    stats: &[StatOverlaySpec],
    metrics: &Metrics,
    fps: u32,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, AppError> {
    let work_dir = std::env::temp_dir().join(format!("rideviz-video-{}", Uuid::new_v4()));
    fs::create_dir_all(&work_dir).map_err(|err| {
        AppError::Internal(format!("Failed to create video temp directory: {}", err))
    })?;

    let result = (|| -> Result<Vec<u8>, AppError> {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Internal("MP4 export cancelled".to_string()));
        }

        let precomputed = render::precompute_route_3d(data, options)
            .map_err(|e| AppError::Internal(format!("Failed to precompute route geometry: {}", e)))?;

        let t_frames_start = std::time::Instant::now();
        for idx in 0..options.animation_frames {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Internal("MP4 export cancelled".to_string()));
            }
            let linear_progress = if options.animation_frames <= 1 {
                1.0
            } else {
                idx as f64 / (options.animation_frames - 1) as f64
            };
            let progress = progress::map_linear_progress_to_route(data, linear_progress);
            let frame_stats = build_stats_overlay_items_at_progress(stats, data, metrics, progress);
            let svg = render::render_svg_frame_precomputed(&precomputed, options, progress, &frame_stats)
                .map_err(|e| AppError::Internal(format!("Failed to render frame {}: {}", idx, e)))?;
            let png_bytes = rasterize::rasterize(&svg, output)?;
            let frame_path = frame_file_path(&work_dir, idx);
            fs::write(&frame_path, png_bytes).map_err(|err| {
                AppError::Internal(format!(
                    "Failed to write video frame {} ({}): {}",
                    idx,
                    frame_path.display(),
                    err
                ))
            })?;
        }
        tracing::info!(
            "Rendered {} frames in {:.2}s ({:.0}ms/frame)",
            options.animation_frames,
            t_frames_start.elapsed().as_secs_f64(),
            t_frames_start.elapsed().as_millis() as f64 / options.animation_frames.max(1) as f64
        );

        let frame_pattern = work_dir.join("frame_%05d.png");
        let output_path = work_dir.join("rideviz-route.mp4");
        let t_ffmpeg = std::time::Instant::now();
        encode_frames_to_mp4(&frame_pattern, &output_path, fps, cancel)?;
        tracing::info!("ffmpeg encode took {:.2}s", t_ffmpeg.elapsed().as_secs_f64());

        fs::read(&output_path).map_err(|err| {
            AppError::Internal(format!(
                "Failed to read encoded MP4 ({}): {}",
                output_path.display(),
                err
            ))
        })
    })();

    let _ = fs::remove_dir_all(&work_dir);
    result
}

fn frame_file_path(work_dir: &FsPath, idx: u32) -> PathBuf {
    work_dir.join(format!("frame_{idx:05}.png"))
}

fn encode_frames_to_mp4(
    frame_pattern: &FsPath,
    output_path: &FsPath,
    fps: u32,
    cancel: &AtomicBool,
) -> Result<(), AppError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Internal("MP4 export cancelled".to_string()));
    }

    let mut child = Command::new("ffmpeg")
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-framerate")
        .arg(fps.to_string())
        .arg("-i")
        .arg(frame_pattern)
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("veryfast")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .map_err(|err| AppError::Internal(format!("Failed to start ffmpeg: {}", err)))?;

    let mut stderr_reader = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Internal("Failed to capture ffmpeg stderr".to_string()))?;
    let stderr_handle = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr_reader, &mut buf);
        buf
    });

    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::Internal("MP4 export cancelled".to_string()));
        }

        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(err) => return Err(AppError::Internal(format!("Failed while waiting for ffmpeg: {}", err))),
        }
    };

    let stderr = stderr_handle
        .join()
        .unwrap_or_else(|_| "Failed to read ffmpeg stderr".to_string())
        .trim()
        .to_string();

    if status.success() {
        return Ok(());
    }

    Err(AppError::Internal(format!(
        "ffmpeg failed to encode MP4: {}",
        if stderr.is_empty() { "unknown error" } else { &stderr }
    )))
}

async fn route_data(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(query): Query<RouteDataQuery>,
) -> Result<Json<RouteDataResponse>, AppError> {
    let processed = state
        .get(&file_id)
        .ok_or_else(|| AppError::NotFound(file_id.clone()))?;

    let mut options = RenderOptions::route_3d_defaults();
    options.smoothing = query.smoothing;
    let (simplify, curve_tension) = smoothing_to_route_params(query.smoothing);
    options.simplify = simplify;
    options.curve_tension = curve_tension;
    options.color_by = query
        .color_by
        .as_deref()
        .and_then(ColorByMetric::from_str);

    let viz_data = prepare::prepare(&processed, &options)?;

    Ok(Json(RouteDataResponse {
        file_id,
        viz_data,
        metrics: processed.metrics,
        available_data: processed.available_data,
    }))
}

fn bearer_token(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers.get("authorization")?;
    let raw = value.to_str().ok()?;
    raw.strip_prefix("Bearer ").map(|token| token.trim().to_string())
}
