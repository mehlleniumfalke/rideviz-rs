use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorByMetric {
    Elevation,
    Speed,
    #[serde(rename = "heartrate")]
    HeartRate,
    Power,
}

impl ColorByMetric {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "elevation" => Some(Self::Elevation),
            "speed" => Some(Self::Speed),
            "heartrate" | "heart_rate" => Some(Self::HeartRate),
            "power" => Some(Self::Power),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationEasing {
    EaseInOutSine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Apng,
    Webm,
}

#[derive(Debug, Clone)]
pub struct VizData {
    pub points: Vec<RoutePoint>,
}

#[derive(Debug, Clone)]
pub struct RoutePoint {
    pub x: f64,
    pub y: f64,
    pub value: Option<f64>,
    pub elevation: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub stroke_width: f32,
    pub gradient: crate::types::gradient::Gradient,
    pub color_by: Option<ColorByMetric>,
    pub smoothing: usize,
    pub glow: bool,
    pub animation_frames: u32,
    pub animation_duration_ms: u32,
    pub animation_easing: AnimationEasing,
    /// Catmull-Rom curve tension for route smoothing.
    /// 0.0 = straight lines, 0.5 = very rounded. Good range: 0.2–0.4.
    pub curve_tension: f32,
    /// Keep every Nth point before rendering. Higher = fewer points = smoother but less detailed.
    /// 1 = no simplification, 5 = keep every 5th point. Good range: 3–10.
    pub simplify: usize,
}

impl RenderOptions {
    pub fn route_3d_defaults() -> Self {
        Self {
            width: 1920,
            height: 1080,
            padding: 40,
            stroke_width: 3.0,
            gradient: crate::types::gradient::Gradient::default(),
            color_by: None,
            smoothing: 30,
            glow: true,
            animation_frames: 100,
            animation_duration_ms: 4600,
            animation_easing: AnimationEasing::EaseInOutSine,
            curve_tension: 0.3,
            simplify: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub background: Option<(u8, u8, u8, u8)>,
    pub watermark: bool,
}
