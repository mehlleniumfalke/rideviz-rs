use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::pipeline::{prepare, rasterize, render};
use crate::state::AppState;
use crate::types::{gradient::Gradient, viz::{OutputConfig, RenderOptions, VizType}};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/visualize", post(visualize))
}

#[derive(Deserialize, Serialize)]
struct VisualizeRequest {
    file_id: String,
    #[serde(rename = "type")]
    viz_type: String,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default = "default_gradient")]
    gradient: String,
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default = "default_stroke_width")]
    stroke_width: f32,
    #[serde(default = "default_padding")]
    padding: u32,
    #[serde(default = "default_smoothing")]
    smoothing: usize,
    #[serde(default = "default_background")]
    background: String,
    #[serde(default = "default_true")]
    glow: bool,
    #[serde(default = "default_true")]
    show_endpoints: bool,
}

fn default_format() -> String {
    "story".to_string()
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

/// Maps smoothing level (0–100) to internal route rendering parameters.
/// Returns (simplify stride, curve tension).
///   0   → raw GPS, straight lines
///   30  → default, balanced
///   100 → heavily stylized, very rounded
fn smoothing_to_route_params(level: usize) -> (usize, f32) {
    let t = level.min(100) as f32 / 100.0;
    let simplify = (1.0 + t * 29.0).round() as usize; // 1 → 30
    let tension = t * 0.45;                             // 0.0 → 0.45
    (simplify, tension)
}

fn default_background() -> String {
    "transparent".to_string()
}

fn default_true() -> bool {
    true
}


async fn visualize(
    State(state): State<AppState>,
    Json(req): Json<VisualizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let processed = state
        .get(&req.file_id)
        .ok_or_else(|| AppError::NotFound(req.file_id.clone()))?;

    let viz_type = VizType::from_str(&req.viz_type)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid visualization type: {}", req.viz_type)))?;

    let mut options = if req.format == "custom" {
        let width = req.width.ok_or_else(|| {
            AppError::BadRequest("width is required for custom format".to_string())
        })?;
        let height = req.height.ok_or_else(|| {
            AppError::BadRequest("height is required for custom format".to_string())
        })?;

        RenderOptions {
            width,
            height,
            padding: req.padding,
            stroke_width: req.stroke_width,
            gradient: Gradient::default(),
            smoothing: req.smoothing,
            glow: req.glow,
            show_endpoints: req.show_endpoints,
            curve_tension: 0.3,
            simplify: 5,
        }
    } else {
        RenderOptions::from_format(&req.format).ok_or_else(|| {
            AppError::BadRequest(format!(
                "Invalid format: {}. Use 'story', 'post', 'wide', or 'custom'",
                req.format
            ))
        })?
    };

    options.gradient = Gradient::get(&req.gradient).unwrap_or_else(Gradient::default);
    options.stroke_width = req.stroke_width;
    options.padding = req.padding;
    options.smoothing = req.smoothing;
    options.glow = req.glow;
    options.show_endpoints = req.show_endpoints;
    let (simplify, curve_tension) = smoothing_to_route_params(req.smoothing);
    options.simplify = simplify;
    options.curve_tension = curve_tension;

    tracing::info!(
        "Generating {} visualization for file {} ({}x{}, gradient: {})",
        viz_type.as_str(),
        req.file_id,
        options.width,
        options.height,
        options.gradient.name
    );

    let viz_data = prepare::prepare(&processed, viz_type)?;
    let svg = render::render_svg(&viz_data, &options)?;

    let background = if req.background == "transparent" {
        None
    } else if req.background.starts_with('#') {
        parse_hex_color(&req.background)
    } else {
        None
    };

    let output_config = OutputConfig {
        width: options.width,
        height: options.height,
        background,
    };

    let png_bytes = rasterize::rasterize(&svg, &output_config)?;

    tracing::info!("Generated PNG: {} bytes", png_bytes.len());

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/png")],
        png_bytes,
    ))
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b, 255))
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        Some((r, g, b, a))
    } else {
        None
    }
}
