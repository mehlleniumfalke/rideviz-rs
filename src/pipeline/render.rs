use crate::error::RenderError;
use crate::types::viz::{RenderOptions, VizData};

pub fn render_svg(data: &VizData, options: &RenderOptions) -> Result<String, RenderError> {
    match data {
        VizData::Route(points) => render_route(points, options),
        VizData::Elevation(points) => render_elevation(points, options),
        VizData::HeartRate(points) => render_time_series(points, options, "Heart Rate"),
        VizData::Power(points) => render_time_series(points, options, "Power"),
    }
}

fn render_route(
    points: &[crate::types::viz::RoutePoint],
    options: &RenderOptions,
) -> Result<String, RenderError> {
    let width = options.width as f64;
    let height = options.height as f64;
    let padding = options.padding as f64;

    let view_width = width - 2.0 * padding;
    let view_height = height - 2.0 * padding;

    let gradient_id = "routeGradient";
    let gradient_def = create_linear_gradient(gradient_id, &options.gradient);

    let mut path_data = String::new();
    for (i, point) in points.iter().enumerate() {
        let x = padding + point.x * view_width;
        let y = padding + (1.0 - point.y) * view_height;

        if i == 0 {
            path_data.push_str(&format!("M {:.2} {:.2}", x, y));
        } else {
            path_data.push_str(&format!(" L {:.2} {:.2}", x, y));
        }
    }

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <defs>
    {}
  </defs>
  <path d="{}" fill="none" stroke="url(#{})" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#,
        width, height, width, height, gradient_def, path_data, gradient_id, options.stroke_width
    ))
}

fn render_elevation(
    points: &[crate::types::viz::ElevationPoint],
    options: &RenderOptions,
) -> Result<String, RenderError> {
    if points.is_empty() {
        return Err(RenderError::SvgError("No elevation points".to_string()));
    }

    let width = options.width as f64;
    let height = options.height as f64;
    let padding = options.padding as f64;

    let view_width = width - 2.0 * padding;
    let view_height = height - 2.0 * padding;

    let max_distance = points.iter().map(|p| p.distance_km).fold(0.0, f64::max);
    let min_elevation = points.iter().map(|p| p.elevation_m).fold(f64::INFINITY, f64::min);
    let max_elevation = points
        .iter()
        .map(|p| p.elevation_m)
        .fold(f64::NEG_INFINITY, f64::max);

    let elevation_range = max_elevation - min_elevation;
    if elevation_range == 0.0 {
        return Err(RenderError::SvgError("No elevation variation".to_string()));
    }

    let gradient_id = "elevationGradient";
    let gradient_def = create_linear_gradient(gradient_id, &options.gradient);

    let mut line_path = String::new();
    let mut area_path = String::new();

    for (i, point) in points.iter().enumerate() {
        let x = padding + (point.distance_km / max_distance) * view_width;
        let y = padding + view_height - ((point.elevation_m - min_elevation) / elevation_range) * view_height;

        if i == 0 {
            line_path.push_str(&format!("M {:.2} {:.2}", x, y));
            area_path.push_str(&format!("M {:.2} {:.2}", x, height - padding));
            area_path.push_str(&format!(" L {:.2} {:.2}", x, y));
        } else {
            line_path.push_str(&format!(" L {:.2} {:.2}", x, y));
            area_path.push_str(&format!(" L {:.2} {:.2}", x, y));
        }
    }

    let last_x = padding + view_width;
    area_path.push_str(&format!(" L {:.2} {:.2} Z", last_x, height - padding));

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <defs>
    {}
  </defs>
  <path d="{}" fill="url(#{})" fill-opacity="0.3"/>
  <path d="{}" fill="none" stroke="url(#{})" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#,
        width,
        height,
        width,
        height,
        gradient_def,
        area_path,
        gradient_id,
        line_path,
        gradient_id,
        options.stroke_width
    ))
}

fn render_time_series(
    points: &[crate::types::viz::TimeSeriesPoint],
    options: &RenderOptions,
    _label: &str,
) -> Result<String, RenderError> {
    if points.is_empty() {
        return Err(RenderError::SvgError("No time series points".to_string()));
    }

    let smoothed = smooth_points(points, options.smoothing);

    let width = options.width as f64;
    let height = options.height as f64;
    let padding = options.padding as f64;

    let view_width = width - 2.0 * padding;
    let view_height = height - 2.0 * padding;

    let max_time = smoothed.iter().map(|p| p.time_offset_sec).fold(0.0, f64::max);
    let min_value = smoothed.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let max_value = smoothed.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max);

    let value_range = max_value - min_value;
    if value_range == 0.0 {
        return Err(RenderError::SvgError("No value variation".to_string()));
    }

    let gradient_id = "timeSeriesGradient";
    let gradient_def = create_linear_gradient(gradient_id, &options.gradient);

    let mut path_data = String::new();

    for (i, point) in smoothed.iter().enumerate() {
        let x = padding + (point.time_offset_sec / max_time) * view_width;
        let y = padding + view_height - ((point.value - min_value) / value_range) * view_height;

        if i == 0 {
            path_data.push_str(&format!("M {:.2} {:.2}", x, y));
        } else {
            path_data.push_str(&format!(" L {:.2} {:.2}", x, y));
        }
    }

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <defs>
    {}
  </defs>
  <path d="{}" fill="none" stroke="url(#{})" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#,
        width, height, width, height, gradient_def, path_data, gradient_id, options.stroke_width
    ))
}

fn create_linear_gradient(id: &str, gradient: &crate::types::gradient::Gradient) -> String {
    format!(
        r#"<linearGradient id="{}" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" style="stop-color:{};stop-opacity:1" />
      <stop offset="100%" style="stop-color:{};stop-opacity:1" />
    </linearGradient>"#,
        id, gradient.colors[0], gradient.colors[1]
    )
}

fn smooth_points(
    points: &[crate::types::viz::TimeSeriesPoint],
    window_size: usize,
) -> Vec<crate::types::viz::TimeSeriesPoint> {
    if window_size <= 1 || points.len() < window_size {
        return points.to_vec();
    }

    let half_window = window_size / 2;
    let mut smoothed = Vec::with_capacity(points.len());

    for i in 0..points.len() {
        let start = i.saturating_sub(half_window);
        let end = (i + half_window + 1).min(points.len());

        let sum: f64 = points[start..end].iter().map(|p| p.value).sum();
        let count = (end - start) as f64;
        let avg = sum / count;

        smoothed.push(crate::types::viz::TimeSeriesPoint {
            time_offset_sec: points[i].time_offset_sec,
            value: avg,
        });
    }

    smoothed
}
