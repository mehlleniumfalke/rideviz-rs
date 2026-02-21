use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::error::AppError;
use crate::pipeline::{animate, prepare, rasterize, render};
use crate::state::AppState;
use crate::types::{
    activity::{AvailableData, Metrics},
    gradient::Gradient,
    viz::{ColorByMetric, OutputConfig, OutputFormat, RenderOptions, StatOverlayItem},
};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/visualize", post(visualize))
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
    
    let output_config = OutputConfig {
        width: options.width,
        height: options.height,
        background,
        watermark: req.watermark,
    };

    let is_static = req.duration_seconds.is_none() && req.animation_frames.is_none() && req.animation_duration_ms.is_none();
    let image_bytes = if is_static {
        // Static image - render single frame at progress=1.0 (full route)
        tracing::info!(
            "Generating static route-3d image for file {} ({}x{}, gradient: {})",
            req.file_id,
            options.width,
            options.height,
            options.gradient.name
        );
        let svg = render::render_svg_frame(&viz_data, &options, 1.0, &stats_overlay)?;
        rasterize::rasterize(&svg, &output_config)?
    } else {
        // Animated output
        tracing::info!(
            "Generating route-3d animation for file {} ({}x{}, gradient: {}, format: {:?})",
            req.file_id,
            options.width,
            options.height,
            options.gradient.name,
            req.format
        );
        animate::render_apng(&viz_data, &options, &output_config, &stats_overlay)?
    };
    
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
