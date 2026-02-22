use crate::error::PrepareError;
use crate::types::activity::{Metrics, ProcessedActivity, TrackPoint};
use crate::types::viz::{ColorByMetric, RenderOptions, RoutePoint, VizData};

pub fn prepare(processed: &ProcessedActivity, options: &RenderOptions) -> Result<VizData, PrepareError> {
    if !processed.available_data.has_coordinates {
        return Err(PrepareError::MissingData("coordinates"));
    }
    if !processed.available_data.has_elevation {
        return Err(PrepareError::MissingData("elevation"));
    }
    if let Some(metric) = options.color_by {
        match metric {
            ColorByMetric::Elevation if !processed.available_data.has_elevation => {
                return Err(PrepareError::MissingData("elevation"));
            }
            ColorByMetric::HeartRate if !processed.available_data.has_heart_rate => {
                return Err(PrepareError::MissingData("heart rate"));
            }
            ColorByMetric::Power if !processed.available_data.has_power => {
                return Err(PrepareError::MissingData("power"));
            }
            ColorByMetric::Speed if !has_speed_samples(&processed.points) => {
                return Err(PrepareError::MissingData("timestamp"));
            }
            _ => {}
        }
    }

    let projected: Vec<(f64, f64)> = processed
        .points
        .iter()
        .map(|p| mercator_project(p.lat, p.lon))
        .collect();

    if projected.is_empty() {
        return Err(PrepareError::MissingData("coordinates"));
    }

    let normalized = normalize_route_points(&projected);
    let values = options
        .color_by
        .map(|metric| compute_route_metric_values(&processed.points, metric));
    let telemetry = compute_route_telemetry(&processed.points, &processed.metrics);

    let points = normalized
        .into_iter()
        .enumerate()
        .map(|(idx, (x, y))| RoutePoint {
            route_progress: telemetry
                .get(idx)
                .map(|sample| sample.route_progress)
                .unwrap_or(0.0),
            cumulative_distance_km: telemetry
                .get(idx)
                .map(|sample| sample.cumulative_distance_km)
                .unwrap_or(0.0),
            cumulative_elevation_gain_m: telemetry
                .get(idx)
                .map(|sample| sample.cumulative_elevation_gain_m)
                .unwrap_or(0.0),
            elapsed_seconds: telemetry.get(idx).and_then(|sample| sample.elapsed_seconds),
            heart_rate: telemetry.get(idx).and_then(|sample| sample.heart_rate),
            power: telemetry.get(idx).and_then(|sample| sample.power),
            cumulative_avg_heart_rate: telemetry
                .get(idx)
                .and_then(|sample| sample.cumulative_avg_heart_rate),
            cumulative_max_heart_rate: telemetry
                .get(idx)
                .and_then(|sample| sample.cumulative_max_heart_rate),
            cumulative_avg_power: telemetry
                .get(idx)
                .and_then(|sample| sample.cumulative_avg_power),
            cumulative_max_power: telemetry
                .get(idx)
                .and_then(|sample| sample.cumulative_max_power),
            x,
            y,
            value: values
                .as_ref()
                .and_then(|metric_values| metric_values.get(idx))
                .copied()
                .flatten(),
            elevation: processed.points.get(idx).and_then(|p| p.elevation),
        })
        .collect();

    Ok(VizData { points })
}

fn mercator_project(lat: f64, lon: f64) -> (f64, f64) {
    let x = lon;
    let y = (lat.to_radians().tan() + (1.0 / lat.to_radians().cos())).ln();
    (x, y)
}

fn normalize_route_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.is_empty() {
        return Vec::new();
    }

    let min_x = points.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = points.iter().map(|(x, _)| *x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_y = points.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max);

    let range_x = max_x - min_x;
    let range_y = max_y - min_y;

    if range_x == 0.0 || range_y == 0.0 {
        return points.to_vec();
    }

    points
        .iter()
        .map(|(x, y)| ((*x - min_x) / range_x, (*y - min_y) / range_y))
        .collect()
}

fn compute_route_metric_values(points: &[TrackPoint], metric: ColorByMetric) -> Vec<Option<f64>> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut values = vec![None; points.len()];

    match metric {
        ColorByMetric::Elevation => {
            const SMOOTH_WINDOW: usize = 5;
            const MAX_GRADE: f64 = 0.15;

            // Compute raw per-segment grades
            let mut raw_grades = vec![0.0_f64; points.len()];
            for i in 0..points.len().saturating_sub(1) {
                let current = &points[i];
                let next = &points[i + 1];
                if let (Some(curr_elev), Some(next_elev)) = (current.elevation, next.elevation) {
                    let distance_km =
                        haversine_distance(current.lat, current.lon, next.lat, next.lon);
                    if distance_km > f64::EPSILON {
                        raw_grades[i] = (next_elev - curr_elev) / (distance_km * 1000.0);
                    }
                }
            }

            // Smooth over a sliding window and clip to a realistic grade range
            for i in 0..points.len() {
                let start = i.saturating_sub(SMOOTH_WINDOW);
                let end = (i + SMOOTH_WINDOW + 1).min(points.len());
                let count = (end - start) as f64;
                let avg = raw_grades[start..end].iter().sum::<f64>() / count;
                values[i] = Some(avg.clamp(-MAX_GRADE, MAX_GRADE));
            }
        }
        ColorByMetric::Speed => {
            for i in 0..points.len().saturating_sub(1) {
                let current = &points[i];
                let next = &points[i + 1];
                if let (Some(current_time), Some(next_time)) = (current.time, next.time) {
                    let delta_seconds = (next_time - current_time).num_seconds() as f64;
                    if delta_seconds > f64::EPSILON {
                        let distance_km =
                            haversine_distance(current.lat, current.lon, next.lat, next.lon);
                        let speed_kmh = distance_km / (delta_seconds / 3600.0);
                        values[i] = Some(speed_kmh);
                    }
                }
            }
        }
        ColorByMetric::HeartRate => {
            for (idx, point) in points.iter().enumerate() {
                values[idx] = point.heart_rate.map(|hr| hr as f64);
            }
        }
        ColorByMetric::Power => {
            for (idx, point) in points.iter().enumerate() {
                values[idx] = point.power.map(|power| power as f64);
            }
        }
    }

    if values.len() >= 2 && values[values.len() - 1].is_none() {
        let last_idx = values.len() - 1;
        let prev_idx = last_idx - 1;
        values[last_idx] = values[prev_idx];
    }

    normalize_optional_values(&values)
}

fn normalize_optional_values(values: &[Option<f64>]) -> Vec<Option<f64>> {
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for value in values.iter().flatten() {
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }

    if !min_value.is_finite() || !max_value.is_finite() {
        return vec![None; values.len()];
    }

    let range = max_value - min_value;
    if range <= f64::EPSILON {
        return values
            .iter()
            .map(|value| value.map(|_| 0.5))
            .collect();
    }

    values
        .iter()
        .map(|value| value.map(|v| (v - min_value) / range))
        .collect()
}

fn has_speed_samples(points: &[TrackPoint]) -> bool {
    points.windows(2).any(|pair| {
        let a = &pair[0];
        let b = &pair[1];
        if let (Some(time_a), Some(time_b)) = (a.time, b.time) {
            return (time_b - time_a).num_seconds() > 0;
        }
        false
    })
}

#[derive(Debug, Clone, Copy)]
struct RouteTelemetrySample {
    route_progress: f64,
    cumulative_distance_km: f64,
    cumulative_elevation_gain_m: f64,
    elapsed_seconds: Option<f64>,
    heart_rate: Option<f64>,
    power: Option<f64>,
    cumulative_avg_heart_rate: Option<f64>,
    cumulative_max_heart_rate: Option<f64>,
    cumulative_avg_power: Option<f64>,
    cumulative_max_power: Option<f64>,
}

fn compute_route_telemetry(points: &[TrackPoint], metrics: &Metrics) -> Vec<RouteTelemetrySample> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut cumulative_distance_raw = 0.0_f64;
    let mut cumulative_gain_raw = 0.0_f64;
    let mut elapsed_raw = 0.0_f64;
    let mut hr_sum = 0_u64;
    let mut hr_count = 0_u32;
    let mut max_hr = 0_u16;
    let mut power_sum = 0_u64;
    let mut power_count = 0_u32;
    let mut max_power = 0_u16;

    let mut raw_distance = Vec::with_capacity(points.len());
    let mut raw_gain = Vec::with_capacity(points.len());
    let mut raw_elapsed = Vec::with_capacity(points.len());
    let mut raw_hr_sum = Vec::with_capacity(points.len());
    let mut raw_hr_count = Vec::with_capacity(points.len());
    let mut raw_max_hr = Vec::with_capacity(points.len());
    let mut raw_power_sum = Vec::with_capacity(points.len());
    let mut raw_power_count = Vec::with_capacity(points.len());
    let mut raw_max_power = Vec::with_capacity(points.len());

    for idx in 0..points.len() {
        if idx > 0 {
            let prev = &points[idx - 1];
            let curr = &points[idx];

            cumulative_distance_raw += haversine_distance(prev.lat, prev.lon, curr.lat, curr.lon);

            if let (Some(prev_ele), Some(curr_ele)) = (prev.elevation, curr.elevation) {
                let gain = curr_ele - prev_ele;
                if gain > 0.0 {
                    cumulative_gain_raw += gain;
                }
            }

            if let (Some(prev_time), Some(curr_time)) = (prev.time, curr.time) {
                elapsed_raw += (curr_time - prev_time).num_seconds().max(0) as f64;
            }
        }

        if let Some(hr) = points[idx].heart_rate {
            hr_sum += hr as u64;
            hr_count += 1;
            max_hr = max_hr.max(hr);
        }
        if let Some(power) = points[idx].power {
            power_sum += power as u64;
            power_count += 1;
            max_power = max_power.max(power);
        }

        raw_distance.push(cumulative_distance_raw);
        raw_gain.push(cumulative_gain_raw);
        raw_elapsed.push(elapsed_raw);
        raw_hr_sum.push(hr_sum);
        raw_hr_count.push(hr_count);
        raw_max_hr.push(max_hr);
        raw_power_sum.push(power_sum);
        raw_power_count.push(power_count);
        raw_max_power.push(max_power);
    }

    let total_distance_raw = raw_distance.last().copied().unwrap_or(0.0);
    let total_gain_raw = raw_gain.last().copied().unwrap_or(0.0);
    let total_elapsed_raw = raw_elapsed.last().copied().unwrap_or(0.0);
    let total_points = points.len();

    (0..points.len())
        .map(|idx| {
            let fallback_progress = if total_points <= 1 {
                1.0
            } else {
                idx as f64 / (total_points - 1) as f64
            };
            let route_progress = if total_distance_raw > f64::EPSILON {
                (raw_distance[idx] / total_distance_raw).clamp(0.0, 1.0)
            } else {
                fallback_progress
            };

            let gain_progress = if total_gain_raw > f64::EPSILON {
                (raw_gain[idx] / total_gain_raw).clamp(0.0, 1.0)
            } else {
                route_progress
            };

            let elapsed_seconds = if metrics.duration_seconds > 0 {
                let elapsed_progress = if total_elapsed_raw > f64::EPSILON {
                    (raw_elapsed[idx] / total_elapsed_raw).clamp(0.0, 1.0)
                } else {
                    route_progress
                };
                Some(elapsed_progress * metrics.duration_seconds as f64)
            } else {
                None
            };

            let cumulative_avg_heart_rate = if raw_hr_count[idx] > 0 {
                Some(raw_hr_sum[idx] as f64 / raw_hr_count[idx] as f64)
            } else {
                None
            };
            let cumulative_avg_power = if raw_power_count[idx] > 0 {
                Some(raw_power_sum[idx] as f64 / raw_power_count[idx] as f64)
            } else {
                None
            };

            RouteTelemetrySample {
                route_progress,
                cumulative_distance_km: route_progress * metrics.distance_km,
                cumulative_elevation_gain_m: gain_progress * metrics.elevation_gain_m,
                elapsed_seconds,
                heart_rate: points[idx].heart_rate.map(|value| value as f64),
                power: points[idx].power.map(|value| value as f64),
                cumulative_avg_heart_rate,
                cumulative_max_heart_rate: if raw_max_hr[idx] > 0 {
                    Some(raw_max_hr[idx] as f64)
                } else {
                    None
                },
                cumulative_avg_power,
                cumulative_max_power: if raw_max_power[idx] > 0 {
                    Some(raw_max_power[idx] as f64)
                } else {
                    None
                },
            }
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
