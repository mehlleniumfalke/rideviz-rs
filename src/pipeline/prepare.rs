use crate::error::PrepareError;
use crate::types::activity::ProcessedActivity;
use crate::types::viz::{ElevationPoint, RoutePoint, TimeSeriesPoint, VizData, VizType};

pub fn prepare(processed: &ProcessedActivity, viz_type: VizType) -> Result<VizData, PrepareError> {
    match viz_type {
        VizType::Route => prepare_route(processed),
        VizType::Elevation => prepare_elevation(processed),
        VizType::HeartRate => prepare_heart_rate(processed),
        VizType::Power => prepare_power(processed),
    }
}

fn prepare_route(processed: &ProcessedActivity) -> Result<VizData, PrepareError> {
    if !processed.available_data.has_coordinates {
        return Err(PrepareError::MissingData("coordinates"));
    }

    let points: Vec<RoutePoint> = processed
        .points
        .iter()
        .map(|p| {
            let (x, y) = mercator_project(p.lat, p.lon);
            RoutePoint { x, y }
        })
        .collect();

    if points.is_empty() {
        return Err(PrepareError::MissingData("coordinates"));
    }

    let normalized = normalize_route_points(&points);
    Ok(VizData::Route(normalized))
}

fn prepare_elevation(processed: &ProcessedActivity) -> Result<VizData, PrepareError> {
    if !processed.available_data.has_elevation {
        return Err(PrepareError::MissingData("elevation"));
    }

    let mut distance_km = 0.0;
    let mut points = Vec::new();

    for i in 0..processed.points.len() {
        let point = &processed.points[i];
        if let Some(elevation) = point.elevation {
            points.push(ElevationPoint {
                distance_km,
                elevation_m: elevation,
            });

            if i + 1 < processed.points.len() {
                let next = &processed.points[i + 1];
                distance_km += haversine_distance(point.lat, point.lon, next.lat, next.lon);
            }
        }
    }

    if points.is_empty() {
        return Err(PrepareError::MissingData("elevation"));
    }

    Ok(VizData::Elevation(points))
}

fn prepare_heart_rate(processed: &ProcessedActivity) -> Result<VizData, PrepareError> {
    if !processed.available_data.has_heart_rate {
        return Err(PrepareError::MissingData("heart rate"));
    }

    let start_time = processed
        .points
        .iter()
        .find_map(|p| p.time)
        .ok_or(PrepareError::MissingData("heart rate"))?;

    let points: Vec<TimeSeriesPoint> = processed
        .points
        .iter()
        .filter_map(|p| {
            let time = p.time?;
            let hr = p.heart_rate?;
            Some(TimeSeriesPoint {
                time_offset_sec: (time - start_time).num_seconds() as f64,
                value: hr as f64,
            })
        })
        .collect();

    if points.is_empty() {
        return Err(PrepareError::MissingData("heart rate"));
    }

    Ok(VizData::HeartRate(points))
}

fn prepare_power(processed: &ProcessedActivity) -> Result<VizData, PrepareError> {
    if !processed.available_data.has_power {
        return Err(PrepareError::MissingData("power"));
    }

    let start_time = processed
        .points
        .iter()
        .find_map(|p| p.time)
        .ok_or(PrepareError::MissingData("power"))?;

    let points: Vec<TimeSeriesPoint> = processed
        .points
        .iter()
        .filter_map(|p| {
            let time = p.time?;
            let power = p.power?;
            Some(TimeSeriesPoint {
                time_offset_sec: (time - start_time).num_seconds() as f64,
                value: power as f64,
            })
        })
        .collect();

    if points.is_empty() {
        return Err(PrepareError::MissingData("power"));
    }

    Ok(VizData::Power(points))
}

fn mercator_project(lat: f64, lon: f64) -> (f64, f64) {
    let x = lon;
    let y = (lat.to_radians().tan() + (1.0 / lat.to_radians().cos())).ln();
    (x, y)
}

fn normalize_route_points(points: &[RoutePoint]) -> Vec<RoutePoint> {
    if points.is_empty() {
        return Vec::new();
    }

    let min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

    let range_x = max_x - min_x;
    let range_y = max_y - min_y;

    if range_x == 0.0 || range_y == 0.0 {
        return points.to_vec();
    }

    points
        .iter()
        .map(|p| RoutePoint {
            x: (p.x - min_x) / range_x,
            y: (p.y - min_y) / range_y,
        })
        .collect()
}

fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    R * c
}
