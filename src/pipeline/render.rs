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

    let stride = options.simplify.max(1);
    let coords: Vec<(f64, f64)> = points
        .iter()
        .enumerate()
        .filter(|(i, _)| i % stride == 0 || *i == points.len() - 1)
        .map(|(_, p)| (padding + p.x * view_width, padding + (1.0 - p.y) * view_height))
        .collect();

    let path_data = if options.curve_tension > 0.0 {
        build_smooth_path(&coords, options.curve_tension)
    } else {
        coords.iter().enumerate().fold(String::new(), |mut s, (i, (x, y))| {
            if i == 0 { s.push_str(&format!("M {:.2} {:.2}", x, y)); }
            else { s.push_str(&format!(" L {:.2} {:.2}", x, y)); }
            s
        })
    };

    let glow_filter = if options.glow {
        r#"<filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="6" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>"#.to_string()
    } else {
        String::new()
    };

    let glow_path = if options.glow {
        format!(
            r#"<path d="{}" fill="none" stroke="url(#{})" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round" filter="url(#glow)" opacity="0.6"/>"#,
            path_data, gradient_id, options.stroke_width * 3.0
        )
    } else {
        String::new()
    };

    let endpoint_dots = if options.show_endpoints && points.len() >= 2 {
        let first = &points[0];
        let last = &points[points.len() - 1];
        let sx = padding + first.x * view_width;
        let sy = padding + (1.0 - first.y) * view_height;
        let ex = padding + last.x * view_width;
        let ey = padding + (1.0 - last.y) * view_height;
        let r = options.stroke_width as f64 * 2.5;
        let start_color = &options.gradient.colors[0];
        let end_color = &options.gradient.colors[options.gradient.colors.len() - 1];
        format!(
            r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" opacity="0.95"/>
  <circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" opacity="0.95"/>"#,
            sx, sy, r, start_color,
            ex, ey, r, end_color
        )
    } else {
        String::new()
    };

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <defs>
    {}
    {}
  </defs>
  {}
  <path d="{}" fill="none" stroke="url(#{})" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>
  {}
</svg>"#,
        width, height, width, height,
        gradient_def, glow_filter,
        glow_path,
        path_data, gradient_id, options.stroke_width,
        endpoint_dots
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

    let glow_filter = if options.glow {
        r#"<filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="6" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>"#.to_string()
    } else {
        String::new()
    };

    let glow_path = if options.glow {
        format!(
            r#"<path d="{}" fill="none" stroke="url(#{})" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round" filter="url(#glow)" opacity="0.6"/>"#,
            line_path, gradient_id, options.stroke_width * 3.0
        )
    } else {
        String::new()
    };

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <defs>
    {}
    {}
  </defs>
  <path d="{}" fill="url(#{})" fill-opacity="0.3"/>
  {}
  <path d="{}" fill="none" stroke="url(#{})" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#,
        width, height, width, height,
        gradient_def, glow_filter,
        area_path, gradient_id,
        glow_path,
        line_path, gradient_id, options.stroke_width
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

    let glow_filter = if options.glow {
        r#"<filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="6" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>"#.to_string()
    } else {
        String::new()
    };

    let glow_path = if options.glow {
        format!(
            r#"<path d="{}" fill="none" stroke="url(#{})" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round" filter="url(#glow)" opacity="0.6"/>"#,
            path_data, gradient_id, options.stroke_width * 3.0
        )
    } else {
        String::new()
    };

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <defs>
    {}
    {}
  </defs>
  {}
  <path d="{}" fill="none" stroke="url(#{})" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"#,
        width, height, width, height,
        gradient_def, glow_filter,
        glow_path,
        path_data, gradient_id, options.stroke_width
    ))
}

/// Catmull-Rom spline converted to cubic bezier curves.
/// tension: 0.0 = straight, ~0.3 = smooth, 0.5 = very rounded.
fn build_smooth_path(points: &[(f64, f64)], tension: f32) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        return format!("M {:.2} {:.2}", points[0].0, points[0].1);
    }

    let t = tension as f64;
    let mut path = format!("M {:.2} {:.2}", points[0].0, points[0].1);

    for i in 0..points.len() - 1 {
        let p0 = if i > 0 { points[i - 1] } else { points[i] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() { points[i + 2] } else { points[i + 1] };

        let cp1x = p1.0 + (p2.0 - p0.0) * t;
        let cp1y = p1.1 + (p2.1 - p0.1) * t;
        let cp2x = p2.0 - (p3.0 - p1.0) * t;
        let cp2y = p2.1 - (p3.1 - p1.1) * t;

        path.push_str(&format!(
            " C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2}",
            cp1x, cp1y, cp2x, cp2y, p2.0, p2.1
        ));
    }

    path
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
