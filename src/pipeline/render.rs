use crate::error::RenderError;
use crate::types::gradient::Gradient;
use crate::types::viz::{RenderOptions, RoutePoint, VizData};

const ELEVATION_GAMMA: f64 = 0.82;
const EXTRUSION_RATIO: f64 = 0.24;
const ELEVATION_RANGE_DIVISOR: f64 = 600.0;
const ELEVATION_SCALE_MIN: f64 = 0.7;
const ELEVATION_SCALE_MAX: f64 = 1.4;
const ISOMETRIC_ANGLE_DEG: f64 = 30.0;
const WALL_FILL_OPACITY: f64 = 0.24;
const WALL_SUBDIVISIONS: usize = 4;
const COLOR_BUCKETS: usize = 48;

#[derive(Clone, Copy)]
struct ProjectedPoint {
    ground: (f64, f64),
    top: (f64, f64),
    value: Option<f64>,
}

pub fn render_svg_frame(
    data: &VizData,
    options: &RenderOptions,
    progress: f64,
) -> Result<String, RenderError> {
    render_route_3d(&data.points, options, progress.clamp(0.0, 1.0))
}

fn render_route_3d(
    points: &[RoutePoint],
    options: &RenderOptions,
    progress: f64,
) -> Result<String, RenderError> {
    let width = options.width as f64;
    let height = options.height as f64;
    let padding = options.padding as f64;
    let view_width = width - 2.0 * padding;
    let view_height = height - 2.0 * padding;
    if view_width <= 0.0 || view_height <= 0.0 {
        return Err(RenderError::SvgError("Invalid viewport size".to_string()));
    }

    let filtered_points = filter_route_points(points, options.simplify)?;
    let (min_elev, max_elev) = route_elevation_bounds(&filtered_points)?;
    let elev_range = (max_elev - min_elev).max(f64::EPSILON);
    let elevation_scale =
        ((max_elev - min_elev) / ELEVATION_RANGE_DIVISOR).clamp(ELEVATION_SCALE_MIN, ELEVATION_SCALE_MAX);
    let extrusion_height = view_height * EXTRUSION_RATIO * elevation_scale;

    let projected = project_to_isometric(
        &filtered_points,
        view_width,
        view_height,
        min_elev,
        elev_range,
        extrusion_height,
    );
    let fitted = fit_to_viewport(&projected, padding, view_width, view_height)?;
    let revealed = reveal_projected_points(&fitted, progress);
    let smoothed = subdivide_projected_catmull(&revealed, options.curve_tension, WALL_SUBDIVISIONS);

    let walls = build_wall_polygons(&smoothed, &options.gradient);
    let (ground_coords, top_coords, top_values) = split_projected_points(&smoothed);

    let top_path = if options.color_by.is_some() {
        build_segment_paths(
            &top_coords,
            Some(&top_values),
            options.stroke_width,
            &options.gradient,
        )
    } else {
        format!(
            r#"<path d="{}" fill="none" stroke="url(#routeGradient)" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            build_route_path(&top_coords, options.curve_tension),
            options.stroke_width
        )
    };

    let ground_path = format!(
        r##"<path d="{}" fill="none" stroke="#FFFFFF" stroke-opacity="0.14" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"/>"##,
        build_route_path(&ground_coords, options.curve_tension),
        (options.stroke_width * 0.9).max(1.0)
    );
    let glow_filter = glow_filter_def(options.glow);
    let glow_path = if options.glow {
        if options.color_by.is_some() {
            let glow_segments = build_segment_paths(
                &top_coords,
                Some(&top_values),
                options.stroke_width * 2.4,
                &options.gradient,
            );
            format!(r#"<g filter="url(#glow)" opacity="0.6">{}</g>"#, glow_segments)
        } else {
            format!(
                r#"<path d="{}" fill="none" stroke="url(#routeGradient)" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round" filter="url(#glow)" opacity="0.6"/>"#,
                build_route_path(&top_coords, options.curve_tension),
                options.stroke_width * 2.4
            )
        }
    } else {
        String::new()
    };
    let endpoint_dots = build_3d_endpoint_dots(&top_coords, options);

    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
  <defs>
    {}
    {}
  </defs>
  {}
  {}
  {}
  {}
  {}
</svg>"#,
        width,
        height,
        width,
        height,
        create_linear_gradient("routeGradient", &options.gradient),
        glow_filter,
        walls,
        ground_path,
        glow_path,
        top_path,
        endpoint_dots
    ))
}

fn filter_route_points(points: &[RoutePoint], simplify: usize) -> Result<Vec<&RoutePoint>, RenderError> {
    let stride = simplify.max(1);
    let filtered: Vec<&RoutePoint> = points
        .iter()
        .enumerate()
        .filter(|(i, _)| i % stride == 0 || *i == points.len() - 1)
        .map(|(_, point)| point)
        .collect();
    if filtered.len() < 2 {
        return Err(RenderError::SvgError(
            "Not enough route points for 3D route".to_string(),
        ));
    }
    Ok(filtered)
}

fn route_elevation_bounds(points: &[&RoutePoint]) -> Result<(f64, f64), RenderError> {
    let valid: Vec<f64> = points.iter().filter_map(|point| point.elevation).collect();
    if valid.is_empty() {
        return Err(RenderError::SvgError(
            "No elevation data available for 3D route".to_string(),
        ));
    }
    let min_elev = valid.iter().copied().fold(f64::INFINITY, f64::min);
    let max_elev = valid.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Ok((min_elev, max_elev))
}

fn project_to_isometric(
    points: &[&RoutePoint],
    view_width: f64,
    view_height: f64,
    min_elev: f64,
    elev_range: f64,
    extrusion_height: f64,
) -> Vec<ProjectedPoint> {
    let angle = ISOMETRIC_ANGLE_DEG.to_radians();
    let sin_angle = angle.sin();
    let cos_angle = angle.cos();

    points
        .iter()
        .map(|point| {
            let x = point.x * view_width;
            let y = (1.0 - point.y) * view_height;
            let ground_x = x * cos_angle + y * sin_angle;
            let ground_y = -x * sin_angle + y * cos_angle;
            let norm_elev = point
                .elevation
                .map(|elevation| (elevation - min_elev) / elev_range)
                .unwrap_or(0.0)
                .powf(ELEVATION_GAMMA);
            let top_y = ground_y - norm_elev * extrusion_height;
            ProjectedPoint {
                ground: (ground_x, ground_y),
                top: (ground_x, top_y),
                value: point.value,
            }
        })
        .collect()
}

fn fit_to_viewport(
    points: &[ProjectedPoint],
    padding: f64,
    view_width: f64,
    view_height: f64,
) -> Result<Vec<ProjectedPoint>, RenderError> {
    if points.is_empty() {
        return Err(RenderError::SvgError("No projected points".to_string()));
    }

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.ground.0).min(point.top.0);
        max_x = max_x.max(point.ground.0).max(point.top.0);
        min_y = min_y.min(point.ground.1).min(point.top.1);
        max_y = max_y.max(point.ground.1).max(point.top.1);
    }

    let content_width = (max_x - min_x).max(f64::EPSILON);
    let content_height = (max_y - min_y).max(f64::EPSILON);
    let scale = (view_width / content_width).min(view_height / content_height);
    let offset_x = padding + (view_width - content_width * scale) * 0.5;
    let offset_y = padding + (view_height - content_height * scale) * 0.5;

    Ok(points
        .iter()
        .map(|point| ProjectedPoint {
            ground: (
                offset_x + (point.ground.0 - min_x) * scale,
                offset_y + (point.ground.1 - min_y) * scale,
            ),
            top: (
                offset_x + (point.top.0 - min_x) * scale,
                offset_y + (point.top.1 - min_y) * scale,
            ),
            value: point.value,
        })
        .collect())
}

fn build_wall_polygons(points: &[ProjectedPoint], gradient: &Gradient) -> String {
    let mut walls: Vec<(f64, String)> = Vec::new();
    for i in 0..points.len().saturating_sub(1) {
        let current = points[i];
        let next = points[i + 1];
        let t = current
            .value
            .unwrap_or_else(|| i as f64 / (points.len().saturating_sub(1).max(1)) as f64);
        let color = gradient.interpolate(remap_color_contrast(t));
        let polygon = format!(
            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" fill-opacity="{:.2}"/>"#,
            current.ground.0,
            current.ground.1,
            current.top.0,
            current.top.1,
            next.top.0,
            next.top.1,
            next.ground.0,
            next.ground.1,
            color,
            WALL_FILL_OPACITY
        );
        walls.push((((current.ground.1 + next.ground.1) * 0.5), polygon));
    }
    walls.sort_by(|a, b| a.0.total_cmp(&b.0));
    walls.into_iter().map(|(_, svg)| svg).collect()
}

fn split_projected_points(
    points: &[ProjectedPoint],
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<Option<f64>>) {
    let ground_coords = points.iter().map(|point| point.ground).collect();
    let top_coords = points.iter().map(|point| point.top).collect();
    let top_values = points.iter().map(|point| point.value).collect();
    (ground_coords, top_coords, top_values)
}

fn build_route_path(coords: &[(f64, f64)], curve_tension: f32) -> String {
    if curve_tension > 0.0 {
        build_smooth_path(coords, curve_tension)
    } else {
        build_polyline_path(coords)
    }
}

fn build_3d_endpoint_dots(top_coords: &[(f64, f64)], options: &RenderOptions) -> String {
    if top_coords.len() < 2 {
        return String::new();
    }
    let start = top_coords[0];
    let end = top_coords[top_coords.len() - 1];
    let radius = options.stroke_width as f64 * 2.2;
    render_endpoint_dots(
        start,
        end,
        radius,
        &options.gradient.colors[0],
        &options.gradient.colors[1],
        0.95,
    )
}

fn glow_filter_def(enabled: bool) -> String {
    if !enabled {
        return String::new();
    }
    r#"<filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="6" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>"#
        .to_string()
}

fn render_endpoint_dots(
    start: (f64, f64),
    end: (f64, f64),
    radius: f64,
    start_color: &str,
    end_color: &str,
    end_opacity: f64,
) -> String {
    format!(
        r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" opacity="0.95"/>
  <circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}" opacity="{:.2}"/>"#,
        start.0, start.1, radius, start_color, end.0, end.1, radius, end_color, end_opacity
    )
}

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

fn build_polyline_path(points: &[(f64, f64)]) -> String {
    points.iter().enumerate().fold(String::new(), |mut s, (i, (x, y))| {
        if i == 0 {
            s.push_str(&format!("M {:.2} {:.2}", x, y));
        } else {
            s.push_str(&format!(" L {:.2} {:.2}", x, y));
        }
        s
    })
}

fn create_linear_gradient(id: &str, gradient: &Gradient) -> String {
    format!(
        r#"<linearGradient id="{}" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" style="stop-color:{};stop-opacity:1" />
      <stop offset="100%" style="stop-color:{};stop-opacity:1" />
    </linearGradient>"#,
        id, gradient.colors[0], gradient.colors[1]
    )
}

fn build_segment_paths(
    coords: &[(f64, f64)],
    values: Option<&[Option<f64>]>,
    stroke_width: f32,
    gradient: &Gradient,
) -> String {
    if coords.len() < 2 {
        return String::new();
    }
    let mut bucket_commands = vec![String::new(); COLOR_BUCKETS];
    for i in 0..coords.len() - 1 {
        let (x1, y1) = coords[i];
        let (x2, y2) = coords[i + 1];
        let fallback_t = i as f64 / (coords.len() - 1) as f64;
        let color_t = values
            .and_then(|all_values| all_values.get(i))
            .copied()
            .flatten()
            .map(remap_color_contrast)
            .unwrap_or(fallback_t);
        let bucket_idx = ((color_t * (COLOR_BUCKETS - 1) as f64).round() as usize).min(COLOR_BUCKETS - 1);
        bucket_commands[bucket_idx].push_str(&format!(" M {:.2} {:.2} L {:.2} {:.2}", x1, y1, x2, y2));
    }

    let mut paths = String::new();
    for (bucket_idx, commands) in bucket_commands.into_iter().enumerate() {
        if commands.is_empty() {
            continue;
        }
        let color_t = bucket_idx as f64 / (COLOR_BUCKETS - 1).max(1) as f64;
        paths.push_str(&format!(
            r#"<path d="{}" fill="none" stroke="{}" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"/>"#,
            commands.trim(),
            gradient.interpolate(color_t),
            stroke_width
        ));
    }
    paths
}

fn remap_color_contrast(value: f64) -> f64 {
    let v = value.clamp(0.0, 1.0);
    ((v - 0.5) * 1.55 + 0.5).clamp(0.0, 1.0)
}

fn reveal_projected_points(points: &[ProjectedPoint], progress: f64) -> Vec<ProjectedPoint> {
    if points.len() <= 1 {
        return points.to_vec();
    }
    let progress = progress.clamp(0.0, 1.0);
    if progress >= 1.0 {
        return points.to_vec();
    }

    let segment_lengths: Vec<f64> = points
        .windows(2)
        .map(|pair| distance_2d(pair[0].top, pair[1].top))
        .collect();
    let total_length: f64 = segment_lengths.iter().sum();
    if total_length <= f64::EPSILON {
        return vec![points[0]];
    }

    let target_length = total_length * progress;
    let mut traveled = 0.0;
    let mut out = vec![points[0]];
    for (idx, segment_length) in segment_lengths.iter().copied().enumerate() {
        if segment_length <= f64::EPSILON {
            continue;
        }
        let next_traveled = traveled + segment_length;
        if next_traveled < target_length {
            out.push(points[idx + 1]);
            traveled = next_traveled;
            continue;
        }
        let local_t = ((target_length - traveled) / segment_length).clamp(0.0, 1.0);
        out.push(ProjectedPoint {
            ground: lerp_point(points[idx].ground, points[idx + 1].ground, local_t),
            top: lerp_point(points[idx].top, points[idx + 1].top, local_t),
            value: lerp_optional(points[idx].value, points[idx + 1].value, local_t),
        });
        return out;
    }
    points.to_vec()
}

fn distance_2d(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}

fn lerp_point(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn lerp_optional(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x + (y - x) * t),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

fn subdivide_projected_catmull(
    points: &[ProjectedPoint],
    tension: f32,
    subdivisions: usize,
) -> Vec<ProjectedPoint> {
    if points.len() < 3 || subdivisions < 2 {
        return points.to_vec();
    }
    let curvature = (tension.clamp(0.0, 0.5) as f64 * 2.0).clamp(0.0, 1.0);
    let mut out = Vec::with_capacity((points.len() - 1) * subdivisions + 1);

    for i in 0..points.len() - 1 {
        let p0 = if i > 0 { points[i - 1] } else { points[i] };
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = if i + 2 < points.len() { points[i + 2] } else { points[i + 1] };
        for step in 0..subdivisions {
            if i > 0 && step == 0 {
                continue;
            }
            let t = step as f64 / subdivisions as f64;
            out.push(ProjectedPoint {
                ground: catmull_rom_point(p0.ground, p1.ground, p2.ground, p3.ground, t, curvature),
                top: catmull_rom_point(p0.top, p1.top, p2.top, p3.top, t, curvature),
                value: catmull_rom_optional(p0.value, p1.value, p2.value, p3.value, t, curvature),
            });
        }
    }
    out.push(*points.last().unwrap_or(&points[0]));
    out
}

fn catmull_rom_point(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    t: f64,
    curvature: f64,
) -> (f64, f64) {
    (
        catmull_rom_scalar(p0.0, p1.0, p2.0, p3.0, t, curvature),
        catmull_rom_scalar(p0.1, p1.1, p2.1, p3.1, t, curvature),
    )
}

fn catmull_rom_scalar(p0: f64, p1: f64, p2: f64, p3: f64, t: f64, curvature: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    let m1 = (p2 - p0) * 0.5 * curvature;
    let m2 = (p3 - p1) * 0.5 * curvature;
    (2.0 * t3 - 3.0 * t2 + 1.0) * p1
        + (t3 - 2.0 * t2 + t) * m1
        + (-2.0 * t3 + 3.0 * t2) * p2
        + (t3 - t2) * m2
}

fn catmull_rom_optional(
    p0: Option<f64>,
    p1: Option<f64>,
    p2: Option<f64>,
    p3: Option<f64>,
    t: f64,
    curvature: f64,
) -> Option<f64> {
    match (p0, p1, p2, p3) {
        (Some(v0), Some(v1), Some(v2), Some(v3)) => {
            Some(catmull_rom_scalar(v0, v1, v2, v3, t, curvature))
        }
        _ => lerp_optional(p1, p2, t),
    }
}
