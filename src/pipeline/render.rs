use crate::error::RenderError;
use crate::types::gradient::Gradient;
use crate::types::viz::{RenderOptions, RoutePoint, StatOverlayItem, VizData};

const ELEVATION_GAMMA: f64 = 0.82;
const EXTRUSION_RATIO: f64 = 0.24;
const ELEVATION_RANGE_DIVISOR: f64 = 600.0;
const ELEVATION_SCALE_MIN: f64 = 0.7;
const ELEVATION_SCALE_MAX: f64 = 1.4;
const ISOMETRIC_ANGLE_DEG: f64 = 30.0;
const WALL_FILL_OPACITY: f64 = 0.24;
const WALL_SUBDIVISIONS: usize = 4;
const COLOR_BUCKETS: usize = 48;
const LEGACY_WIDE_WIDTH: f64 = 1920.0;
const LEGACY_WIDE_HEIGHT: f64 = 1080.0;

#[derive(Clone, Copy)]
struct ProjectedPoint {
    ground: (f64, f64),
    top: (f64, f64),
    value: Option<f64>,
    route_progress: f64,
}

pub fn render_svg_frame(
    data: &VizData,
    options: &RenderOptions,
    progress: f64,
    stats: &[StatOverlayItem],
) -> Result<String, RenderError> {
    render_route_3d(&data.points, options, progress.clamp(0.0, 1.0), stats)
}

fn render_route_3d(
    points: &[RoutePoint],
    options: &RenderOptions,
    progress: f64,
    stats: &[StatOverlayItem],
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
    // Keep a fixed legacy-wide camera basis (pre multi-format behavior) for all outputs.
    let projection_width = (LEGACY_WIDE_WIDTH - 2.0 * padding).max(1.0);
    let projection_height = (LEGACY_WIDE_HEIGHT - 2.0 * padding).max(1.0);
    let extrusion_height = projection_height * EXTRUSION_RATIO * elevation_scale;

    let projected = project_to_isometric(
        &filtered_points,
        projection_width,
        projection_height,
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
    let outline_path = format!(
        r##"<path d="{}" fill="none" stroke="#FFFFFF" stroke-opacity="0.55" stroke-width="{:.1}" stroke-linecap="round" stroke-linejoin="round"/>"##,
        build_route_path(&top_coords, options.curve_tension),
        options.stroke_width * 1.5
    );

    let has_extent = smoothed.len() >= 2 && {
        let first = smoothed.first().unwrap().top;
        smoothed.iter().any(|p| {
            let dx = p.top.0 - first.0;
            let dy = p.top.1 - first.1;
            dx * dx + dy * dy > 1.0
        })
    };

    let glow_filter = glow_filter_def(options.glow && has_extent);
    let glow_path = if options.glow && has_extent {
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
    let stats_overlay = build_stats_overlay(stats, options);

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
        outline_path,
        glow_path,
        top_path,
        endpoint_dots,
        stats_overlay
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
    projection_width: f64,
    projection_height: f64,
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
            let x = point.x * projection_width;
            let y = (1.0 - point.y) * projection_height;
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
                route_progress: point.route_progress,
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
            route_progress: point.route_progress,
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

fn build_stats_overlay(stats: &[StatOverlayItem], options: &RenderOptions) -> String {
    if stats.is_empty() {
        return String::new();
    }

    let start_x = options.padding as f64 + 14.0;
    let start_y = options.padding as f64 + 28.0;
    let font_size = ((options.height as f64) * 0.024).clamp(12.0, 34.0);
    let line_gap = (font_size * 1.38).clamp(18.0, 52.0);
    let label_dx = (font_size * 6.1).clamp(72.0, 280.0);

    let lines: String = stats
        .iter()
        .enumerate()
        .map(|(idx, stat)| {
            let y = start_y + idx as f64 * line_gap;
            let color = options.gradient.interpolate(stat.color_t);
            format!(
                r#"<text x="{:.2}" y="{:.2}" font-family="Geist Sans, Geist, DejaVu Sans, sans-serif" font-size="{:.2}" font-weight="600" letter-spacing="0.2" fill="{}" fill-opacity="0.78">{}</text>
<text x="{:.2}" y="{:.2}" font-family="Geist Sans, Geist, DejaVu Sans, sans-serif" font-size="{:.2}" font-weight="700" fill="{}">{}</text>"#,
                start_x,
                y,
                font_size * 0.68,
                color,
                stat.label,
                start_x + label_dx,
                y,
                font_size,
                color,
                stat.value
            )
        })
        .collect();

    format!(r#"<g id="statsOverlay">{}</g>"#, lines)
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
    let start_color = options.gradient.colors.first().copied().unwrap_or("#FFFFFF");
    let end_color = options.gradient.colors.last().copied().unwrap_or("#FFFFFF");
    render_endpoint_dots(start, end, radius, start_color, end_color, 0.95)
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
    let stops = &gradient.colors;
    let stop_elements: String = stops
        .iter()
        .enumerate()
        .map(|(i, color)| {
            let offset = if stops.len() == 1 {
                0.0
            } else {
                i as f64 / (stops.len() - 1) as f64 * 100.0
            };
            format!(
                r#"<stop offset="{:.1}%" style="stop-color:{};stop-opacity:1" />"#,
                offset, color
            )
        })
        .collect();
    format!(
        r#"<linearGradient id="{}" x1="0%" y1="0%" x2="100%" y2="0%">
      {}
    </linearGradient>"#,
        id, stop_elements
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
    if progress <= 0.0 {
        return vec![points[0]];
    }
    if progress >= 1.0 {
        return points.to_vec();
    }

    let mut out = vec![points[0]];
    for idx in 0..points.len().saturating_sub(1) {
        let current = points[idx];
        let next = points[idx + 1];
        if next.route_progress <= current.route_progress {
            continue;
        }
        if next.route_progress < progress {
            out.push(next);
            continue;
        }
        let local_t = ((progress - current.route_progress)
            / (next.route_progress - current.route_progress))
            .clamp(0.0, 1.0);
        out.push(ProjectedPoint {
            ground: lerp_point(current.ground, next.ground, local_t),
            top: lerp_point(current.top, next.top, local_t),
            value: lerp_optional(current.value, next.value, local_t),
            route_progress: progress,
        });
        return out;
    }
    points.to_vec()
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

fn lerp_scalar(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
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
                route_progress: lerp_scalar(p1.route_progress, p2.route_progress, t),
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
