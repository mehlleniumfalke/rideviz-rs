use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackPoint {
    pub lat: f64,
    pub lon: f64,
    pub elevation: Option<f64>,
    pub time: Option<DateTime<Utc>>,
    pub heart_rate: Option<u16>,
    pub power: Option<u16>,
    pub cadence: Option<u16>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FileFormat {
    Gpx,
    Fit,
}

impl FileFormat {
    pub fn from_filename(filename: &str) -> Option<Self> {
        let ext = filename.rsplit('.').next()?.to_lowercase();
        match ext.as_str() {
            "gpx" => Some(FileFormat::Gpx),
            "fit" => Some(FileFormat::Fit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedActivity {
    pub points: Vec<TrackPoint>,
    pub file_format: FileFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub distance_km: f64,
    pub elevation_gain_m: f64,
    pub duration_seconds: u64,
    pub avg_speed_kmh: f64,
    pub avg_heart_rate: Option<u16>,
    pub max_heart_rate: Option<u16>,
    pub avg_power: Option<u16>,
    pub max_power: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableData {
    pub has_coordinates: bool,
    pub has_elevation: bool,
    pub has_heart_rate: bool,
    pub has_power: bool,
}

#[derive(Debug, Clone)]
pub struct ProcessedActivity {
    pub points: Vec<TrackPoint>,
    pub metrics: Metrics,
    pub available_data: AvailableData,
}
