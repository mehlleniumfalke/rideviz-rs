use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::pipeline::{animate, prepare, rasterize, render};
use crate::state::AppState;
use crate::types::{
    gradient::Gradient,
    viz::{ColorByMetric, OutputConfig, RenderOptions},
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
    animation_frames: Option<u32>,
    animation_duration_ms: Option<u32>,
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

/// Maps smoothing level (0-100) to internal route rendering parameters.
/// Returns (simplify stride, curve tension).
fn smoothing_to_route_params(level: usize) -> (usize, f32) {
    let t = level.min(100) as f32 / 100.0;
    let simplify = (1.0 + t * 29.0).round() as usize; // 1 -> 30
    let tension = t * 0.45; // 0.0 -> 0.45
    (simplify, tension)
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
    options.animation_frames = req
        .animation_frames
        .unwrap_or(options.animation_frames)
        .clamp(8, 180);
    options.animation_duration_ms = req
        .animation_duration_ms
        .unwrap_or(options.animation_duration_ms)
        .clamp(500, 8000);

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
    };

    let image_bytes = if req.animation_frames.is_none() && req.animation_duration_ms.is_none() {
        // Static image - render single frame at progress=1.0 (full route)
        tracing::info!(
            "Generating static route-3d image for file {} ({}x{}, gradient: {})",
            req.file_id,
            options.width,
            options.height,
            options.gradient.name
        );
        let svg = render::render_svg_frame(&viz_data, &options, 1.0)?;
        rasterize::rasterize(&svg, &output_config)?
    } else {
        // Animated APNG
        tracing::info!(
            "Generating route-3d animation for file {} ({}x{}, gradient: {})",
            req.file_id,
            options.width,
            options.height,
            options.gradient.name
        );
        animate::render_apng(&viz_data, &options, &output_config)?
    };
    
    let (content_type, description) = if req.animation_frames.is_none() && req.animation_duration_ms.is_none() {
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
