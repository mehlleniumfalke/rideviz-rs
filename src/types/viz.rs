use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VizType {
    Route,
    Elevation,
    #[serde(rename = "heartrate")]
    HeartRate,
    Power,
}

impl VizType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "route" => Some(VizType::Route),
            "elevation" => Some(VizType::Elevation),
            "heartrate" | "heart_rate" => Some(VizType::HeartRate),
            "power" => Some(VizType::Power),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            VizType::Route => "route",
            VizType::Elevation => "elevation",
            VizType::HeartRate => "heartrate",
            VizType::Power => "power",
        }
    }
}

#[derive(Debug, Clone)]
pub enum VizData {
    Route(Vec<RoutePoint>),
    Elevation(Vec<ElevationPoint>),
    HeartRate(Vec<TimeSeriesPoint>),
    Power(Vec<TimeSeriesPoint>),
}

#[derive(Debug, Clone)]
pub struct RoutePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct ElevationPoint {
    pub distance_km: f64,
    pub elevation_m: f64,
}

#[derive(Debug, Clone)]
pub struct TimeSeriesPoint {
    pub time_offset_sec: f64,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub stroke_width: f32,
    pub gradient: crate::types::gradient::Gradient,
    pub smoothing: usize,
    pub glow: bool,
    pub show_endpoints: bool,
    /// Catmull-Rom curve tension for route smoothing.
    /// 0.0 = straight lines, 0.5 = very rounded. Good range: 0.2–0.4.
    pub curve_tension: f32,
    /// Keep every Nth point before rendering. Higher = fewer points = smoother but less detailed.
    /// 1 = no simplification, 5 = keep every 5th point. Good range: 3–10.
    pub simplify: usize,
}

impl RenderOptions {
    pub fn from_format(format: &str) -> Option<Self> {
        let (width, height) = match format {
            "story" => (1080, 1920),
            "post" => (1080, 1080),
            "wide" => (1920, 1080),
            _ => return None,
        };

        Some(Self {
            width,
            height,
            padding: 80,
            stroke_width: 4.0,
            gradient: crate::types::gradient::Gradient::default(),
            smoothing: 5,
            glow: true,
            show_endpoints: true,
            curve_tension: 0.3,
            simplify: 5,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub background: Option<(u8, u8, u8, u8)>,
}
