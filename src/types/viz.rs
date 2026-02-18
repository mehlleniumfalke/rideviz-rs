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
            padding: 40,
            stroke_width: 3.0,
            gradient: crate::types::gradient::Gradient::default(),
            smoothing: 5,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub background: Option<(u8, u8, u8, u8)>,
}
