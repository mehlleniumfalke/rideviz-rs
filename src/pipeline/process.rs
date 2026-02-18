use crate::error::ProcessError;
use crate::types::activity::{AvailableData, Metrics, ParsedActivity, ProcessedActivity, TrackPoint};

const MAX_POINTS: usize = 1000;

pub fn process(parsed: &ParsedActivity) -> Result<ProcessedActivity, ProcessError> {
    if parsed.points.len() < 2 {
        return Err(ProcessError::InsufficientPoints(parsed.points.len()));
    }

    let metrics = compute_metrics(&parsed.points);
    let available_data = detect_available_data(&parsed.points);
    let points = downsample(&parsed.points);

    Ok(ProcessedActivity {
        points,
        metrics,
        available_data,
    })
}

fn compute_metrics(points: &[TrackPoint]) -> Metrics {
    let mut distance_km = 0.0;
    let mut elevation_gain_m = 0.0;
    let mut duration_seconds = 0;
    let mut hr_sum = 0u64;
    let mut hr_count = 0;
    let mut max_hr = 0u16;
    let mut power_sum = 0u64;
    let mut power_count = 0;
    let mut max_power = 0u16;

    for i in 1..points.len() {
        let prev = &points[i - 1];
        let curr = &points[i];

        distance_km += haversine_distance(prev.lat, prev.lon, curr.lat, curr.lon);

        if let (Some(prev_ele), Some(curr_ele)) = (prev.elevation, curr.elevation) {
            let gain = curr_ele - prev_ele;
            if gain > 0.0 {
                elevation_gain_m += gain;
            }
        }

        if let (Some(prev_time), Some(curr_time)) = (prev.time, curr.time) {
            duration_seconds += (curr_time - prev_time).num_seconds().max(0) as u64;
        }

        if let Some(hr) = curr.heart_rate {
            hr_sum += hr as u64;
            hr_count += 1;
            max_hr = max_hr.max(hr);
        }

        if let Some(power) = curr.power {
            power_sum += power as u64;
            power_count += 1;
            max_power = max_power.max(power);
        }
    }

    let avg_speed_kmh = if duration_seconds > 0 {
        (distance_km / (duration_seconds as f64)) * 3600.0
    } else {
        0.0
    };

    Metrics {
        distance_km,
        elevation_gain_m,
        duration_seconds,
        avg_speed_kmh,
        avg_heart_rate: if hr_count > 0 {
            Some((hr_sum / hr_count) as u16)
        } else {
            None
        },
        max_heart_rate: if max_hr > 0 { Some(max_hr) } else { None },
        avg_power: if power_count > 0 {
            Some((power_sum / power_count) as u16)
        } else {
            None
        },
        max_power: if max_power > 0 { Some(max_power) } else { None },
    }
}

fn detect_available_data(points: &[TrackPoint]) -> AvailableData {
    let has_coordinates = points.iter().any(|p| p.lat != 0.0 || p.lon != 0.0);
    let has_elevation = points.iter().any(|p| p.elevation.is_some());
    let has_heart_rate = points.iter().any(|p| p.heart_rate.is_some());
    let has_power = points.iter().any(|p| p.power.is_some());

    AvailableData {
        has_coordinates,
        has_elevation,
        has_heart_rate,
        has_power,
    }
}

fn downsample(points: &[TrackPoint]) -> Vec<TrackPoint> {
    if points.len() <= MAX_POINTS {
        return points.to_vec();
    }

    lttb_downsample(points, MAX_POINTS)
}

fn lttb_downsample(data: &[TrackPoint], threshold: usize) -> Vec<TrackPoint> {
    if threshold >= data.len() || threshold == 0 {
        return data.to_vec();
    }

    let mut sampled = Vec::with_capacity(threshold);
    sampled.push(data[0].clone());

    let bucket_size = (data.len() - 2) as f64 / (threshold - 2) as f64;

    let mut a = 0;

    for i in 0..(threshold - 2) {
        let avg_range_start = ((i + 1) as f64 * bucket_size).floor() as usize + 1;
        let avg_range_end = ((i + 2) as f64 * bucket_size).floor() as usize + 1;
        let avg_range_end = avg_range_end.min(data.len());

        let avg_x = (avg_range_start + avg_range_end) as f64 / 2.0;
        let avg_y = data[avg_range_start..avg_range_end]
            .iter()
            .filter_map(|p| p.elevation)
            .sum::<f64>()
            / (avg_range_end - avg_range_start) as f64;

        let range_start = (i as f64 * bucket_size).floor() as usize + 1;
        let range_end = ((i + 1) as f64 * bucket_size).floor() as usize + 1;

        let point_a_x = a as f64;
        let point_a_y = data[a].elevation.unwrap_or(0.0);

        let mut max_area = -1.0;
        let mut max_area_point = range_start;

        for s in range_start..range_end {
            let area = ((point_a_x - avg_x) * (data[s].elevation.unwrap_or(0.0) - point_a_y)
                - (point_a_x - s as f64) * (avg_y - point_a_y))
                .abs();

            if area > max_area {
                max_area = area;
                max_area_point = s;
            }
        }

        sampled.push(data[max_area_point].clone());
        a = max_area_point;
    }

    sampled.push(data[data.len() - 1].clone());
    sampled
}

fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371.0; // Earth radius in km

    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    R * c
}
