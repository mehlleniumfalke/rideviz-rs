use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    f64::consts::PI,
    fs,
    path::{Path as FsPath, PathBuf},
    process::Command,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::license::verify_license_token;
use crate::pipeline::{animate, prepare, rasterize, render};
use crate::state::AppState;
use crate::types::{
    activity::{AvailableData, Metrics},
    gradient::Gradient,
    viz::{ColorByMetric, OutputConfig, OutputFormat, RenderOptions, StatOverlayItem, VizData},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/visualize", post(visualize))
        .route("/api/export/video", post(export_video))
        .route("/api/route-data/:file_id", get(route_data))
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
    duration_seconds: Option<f32>,
    fps: Option<u32>,
    #[serde(default)]
    animation_frames: Option<u32>,
    #[serde(default)]
    animation_duration_ms: Option<u32>,
    #[serde(default = "default_true")]
    watermark: bool,
    #[serde(default)]
    stats: Option<Vec<String>>,
    #[serde(default)]
    format: OutputFormat,
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

fn stat_key_to_overlay(
    key: &str,
    metrics: &Metrics,
    available_data: &AvailableData,
) -> Option<(String, String)> {
    match key {
        "distance" => Some(("DIST".to_string(), format!("{:.1} km", metrics.distance_km))),
        "duration" if metrics.duration_seconds > 0 => {
            Some(("DUR".to_string(), format_duration(metrics.duration_seconds)))
        }
        "elevation_gain" if available_data.has_elevation => {
            Some(("GAIN".to_string(), format!("{:.0} m", metrics.elevation_gain_m)))
        }
        "avg_speed" if metrics.duration_seconds > 0 => {
            Some(("AVG SPD".to_string(), format!("{:.1} km/h", metrics.avg_speed_kmh)))
        }
        "avg_heart_rate" if available_data.has_heart_rate => metrics
            .avg_heart_rate
            .map(|v| ("AVG HR".to_string(), format!("{} bpm", v))),
        "max_heart_rate" if available_data.has_heart_rate => metrics
            .max_heart_rate
            .map(|v| ("MAX HR".to_string(), format!("{} bpm", v))),
        "avg_power" if available_data.has_power => metrics
            .avg_power
            .map(|v| ("AVG PWR".to_string(), format!("{} W", v))),
        "max_power" if available_data.has_power => metrics
            .max_power
            .map(|v| ("MAX PWR".to_string(), format!("{} W", v))),
        _ => None,
    }
}

fn build_stats_overlay_items(
    requested_keys: Option<&Vec<String>>,
    metrics: &Metrics,
    available_data: &AvailableData,
) -> Result<Vec<StatOverlayItem>, AppError> {
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
    let mut items: Vec<(String, String)> = Vec::new();
    for key in keys {
        if !seen.insert(key.to_string()) {
            continue;
        }
        if let Some(entry) = stat_key_to_overlay(key, metrics, available_data) {
            items.push(entry);
        }
    }

    let item_count = items.len().max(1) as f64;
    Ok(items
        .into_iter()
        .enumerate()
        .map(|(idx, (label, value))| {
            let t = if item_count <= 1.0 {
                0.5
            } else {
                idx as f64 / (item_count - 1.0)
            };
            StatOverlayItem {
                label,
                value,
                color_t: t,
            }
        })
        .collect())
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

    let (animation_frames, animation_duration_ms) = if let Some(duration_secs) = req.duration_seconds {
        let duration_secs = duration_secs.clamp(3.0, 60.0);
        let fps = req.fps.unwrap_or(30).clamp(15, 60);
        let frames = (duration_secs * fps as f32).round() as u32;
        let duration_ms = (duration_secs * 1000.0).round() as u32;
        (frames, duration_ms)
    } else {
        let frames = req.animation_frames.unwrap_or(options.animation_frames).clamp(8, 180);
        let duration_ms = req.animation_duration_ms.unwrap_or(options.animation_duration_ms).clamp(500, 8000);
        (frames, duration_ms)
    };

    options.animation_frames = animation_frames;
    options.animation_duration_ms = animation_duration_ms;

    let megapixels = (options.width as f64 * options.height as f64) / 1_000_000.0;
    let frame_ceiling = if megapixels > 6.0 {
        56
    } else if megapixels > 3.0 {
        84
    } else {
        140
    };
    options.animation_frames = options.animation_frames.min(frame_ceiling);

    let (simplify, curve_tension) = smoothing_to_route_params(req.smoothing);
    options.simplify = simplify;
    options.curve_tension = curve_tension;

    let viz_data = prepare::prepare(&processed, &options)?;
    let stats_overlay = build_stats_overlay_items(
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
    
    let is_static = req.duration_seconds.is_none() && req.animation_frames.is_none() && req.animation_duration_ms.is_none();
    let pro_license = bearer_token(&headers)
        .and_then(|token| verify_license_token(&token, &state.config().jwt_secret).ok())
        .map(|claims| claims.pro)
        .unwrap_or(false);

    let output_config = OutputConfig {
        width: options.width,
        height: options.height,
        background,
        watermark: if is_static { !pro_license } else { req.watermark && !pro_license },
    };

    let viz_data_for_render = viz_data.clone();
    let options_for_render = options.clone();
    let output_for_render = output_config.clone();
    let stats_for_render = stats_overlay.clone();
    let image_bytes = tokio::task::spawn_blocking(move || {
        if is_static {
        // Static image - render single frame at progress=1.0 (full route)
            let svg = render::render_svg_frame(
                &viz_data_for_render,
                &options_for_render,
                1.0,
                &stats_for_render,
            )
            .map_err(|err| {
                crate::error::RasterError::RenderFailed(format!(
                    "Failed to render static frame: {}",
                    err
                ))
            })?;
            rasterize::rasterize(&svg, &output_for_render)
        } else {
        // Animated output
            animate::render_apng(&viz_data_for_render, &options_for_render, &output_for_render, &stats_for_render)
        }
    })
    .await
    .map_err(|err| AppError::Internal(format!("Rendering task join failed: {}", err)))??;
    
    let (content_type, description) = if is_static {
        ("image/png", "PNG")
    } else {
        ("image/apng", "APNG")
    };
    
    tracing::info!("Generated {}: {} bytes", description, image_bytes.len());

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        image_bytes,
    ))
}

async fn export_video(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<VideoExportRequest>,
) -> Result<impl IntoResponse, AppError> {
    let token = bearer_token(&headers)
        .ok_or_else(|| AppError::Unauthorized("Missing bearer token".to_string()))?;
    let claims = verify_license_token(&token, &state.config().jwt_secret)
        .map_err(|_| AppError::Unauthorized("Invalid license token".to_string()))?;
    if !claims.pro {
        return Err(AppError::Unauthorized(
            "Pro license required for MP4 export".to_string(),
        ));
    }

    let processed = state
        .get(&req.file_id)
        .ok_or_else(|| AppError::NotFound(req.file_id.clone()))?;

    let mut options = RenderOptions::route_3d_defaults();
    options.gradient = Gradient::get(&req.gradient).unwrap_or_else(Gradient::default);
    match (req.width, req.height) {
        (Some(width), Some(height)) => {
            validate_dimensions(width, height)?;
            if width % 2 != 0 || height % 2 != 0 {
                return Err(AppError::BadRequest(
                    "MP4 export requires even width and height".to_string(),
                ));
            }
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

    let fps = req.fps.clamp(15, 60);
    let duration_seconds = req.duration_seconds.clamp(3.0, 60.0);
    let requested_frame_count = (duration_seconds * fps as f32).round() as u32;
    let frame_count = requested_frame_count.clamp(24, 360);
    options.animation_frames = frame_count;
    options.animation_duration_ms = ((frame_count as f32 / fps as f32) * 1000.0).round() as u32;

    let (simplify, curve_tension) = smoothing_to_route_params(req.smoothing);
    options.simplify = simplify;
    options.curve_tension = curve_tension;

    let background = match req.background.as_deref() {
        Some("white") | None => Some((255, 255, 255, 255)),
        Some("black") => Some((0, 0, 0, 255)),
        Some("transparent") => {
            return Err(AppError::BadRequest(
                "MP4 export does not support transparent background".to_string(),
            ));
        }
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "Invalid background: {}. Use 'white' or 'black'",
                other
            )));
        }
    };

    let viz_data = prepare::prepare(&processed, &options)?;
    let stats_overlay = build_stats_overlay_items(
        req.stats.as_ref(),
        &processed.metrics,
        &processed.available_data,
    )?;
    let output_config = OutputConfig {
        width: options.width,
        height: options.height,
        background,
        watermark: false,
    };

    let video_bytes = tokio::task::spawn_blocking(move || {
        render_mp4_video(&viz_data, &options, &output_config, &stats_overlay, fps)
    })
    .await
    .map_err(|err| AppError::Internal(format!("Video export task join failed: {}", err)))??;

    tracing::info!("Generated MP4: {} bytes", video_bytes.len());

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "video/mp4"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rideviz-route.mp4\"",
            ),
        ],
        video_bytes,
    ))
}

fn render_mp4_video(
    data: &VizData,
    options: &RenderOptions,
    output: &OutputConfig,
    stats: &[StatOverlayItem],
    fps: u32,
) -> Result<Vec<u8>, AppError> {
    let work_dir = std::env::temp_dir().join(format!("rideviz-video-{}", Uuid::new_v4()));
    fs::create_dir_all(&work_dir).map_err(|err| {
        AppError::Internal(format!("Failed to create video temp directory: {}", err))
    })?;

    let result = (|| -> Result<Vec<u8>, AppError> {
        for idx in 0..options.animation_frames {
            let linear_progress = if options.animation_frames <= 1 {
                1.0
            } else {
                idx as f64 / (options.animation_frames - 1) as f64
            };
            let progress = ease_in_out_sine(linear_progress);
            let svg = render::render_svg_frame(data, options, progress, stats)?;
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

        let frame_pattern = work_dir.join("frame_%05d.png");
        let output_path = work_dir.join("rideviz-route.mp4");
        encode_frames_to_mp4(&frame_pattern, &output_path, fps)?;

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

fn encode_frames_to_mp4(frame_pattern: &FsPath, output_path: &FsPath, fps: u32) -> Result<(), AppError> {
    let ffmpeg_output = Command::new("ffmpeg")
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
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-movflags")
        .arg("+faststart")
        .arg(output_path)
        .output()
        .map_err(|err| AppError::Internal(format!("Failed to start ffmpeg: {}", err)))?;

    if ffmpeg_output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&ffmpeg_output.stderr).trim().to_string();
    Err(AppError::Internal(format!(
        "ffmpeg failed to encode MP4: {}",
        if stderr.is_empty() {
            "unknown error".to_string()
        } else {
            stderr
        }
    )))
}

fn ease_in_out_sine(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    0.5 * (1.0 - (PI * t).cos())
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
