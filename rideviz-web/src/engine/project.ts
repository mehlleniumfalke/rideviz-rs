import type { RoutePoint, VizData } from '../types/api';
import type { PreparedPoint, RenderFrameOptions } from './types';

const ELEVATION_GAMMA = 0.82;
const EXTRUSION_RATIO = 0.24;
const ELEVATION_RANGE_DIVISOR = 600;
const ELEVATION_SCALE_MIN = 0.7;
const ELEVATION_SCALE_MAX = 1.4;
const ISOMETRIC_ANGLE_RAD = (30 * Math.PI) / 180;
const LEGACY_WIDE_WIDTH = 1920;
const LEGACY_WIDE_HEIGHT = 1080;

export function prepareFramePoints(data: VizData, options: RenderFrameOptions): PreparedPoint[] {
  if (!data.points.length) return [];
  const simplified = simplifyPoints(data.points, options.smoothing);
  if (simplified.length < 2) return [];

  const elevations = simplified
    .map((point) => point.elevation)
    .filter((value): value is number => typeof value === 'number');
  const minElev = elevations.length ? Math.min(...elevations) : 0;
  const maxElev = elevations.length ? Math.max(...elevations) : 0;
  const elevRange = Math.max(Number.EPSILON, maxElev - minElev);
  const elevScale = clamp((maxElev - minElev) / ELEVATION_RANGE_DIVISOR, ELEVATION_SCALE_MIN, ELEVATION_SCALE_MAX);

  const projectionWidth = Math.max(1, LEGACY_WIDE_WIDTH - options.padding * 2);
  const projectionHeight = Math.max(1, LEGACY_WIDE_HEIGHT - options.padding * 2);
  const extrusionHeight = projectionHeight * EXTRUSION_RATIO * elevScale;
  const cos = Math.cos(ISOMETRIC_ANGLE_RAD);
  const sin = Math.sin(ISOMETRIC_ANGLE_RAD);

  const projected = simplified.map((point) => {
    const x = point.x * projectionWidth;
    const y = (1 - point.y) * projectionHeight;
    const groundX = x * cos + y * sin;
    const groundY = -x * sin + y * cos;
    const normElev = Math.pow(((point.elevation ?? minElev) - minElev) / elevRange, ELEVATION_GAMMA);
    const topY = groundY - normElev * extrusionHeight;
    return {
      groundX,
      groundY,
      topX: groundX,
      topY,
      value: point.value,
      routeProgress: point.route_progress,
    };
  });

  const fitted = fitToViewport(projected, options);
  return revealPoints(fitted, clamp(options.progress, 0, 1));
}

function simplifyPoints(points: RoutePoint[], smoothing: number): RoutePoint[] {
  const stride = Math.max(1, Math.round(1 + (Math.min(100, smoothing) / 100) * 29));
  return points.filter((_, index) => index % stride === 0 || index === points.length - 1);
}

function fitToViewport(points: PreparedPoint[], options: RenderFrameOptions): PreparedPoint[] {
  let minX = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  points.forEach((point) => {
    minX = Math.min(minX, point.groundX, point.topX);
    maxX = Math.max(maxX, point.groundX, point.topX);
    minY = Math.min(minY, point.groundY, point.topY);
    maxY = Math.max(maxY, point.groundY, point.topY);
  });

  const contentWidth = Math.max(Number.EPSILON, maxX - minX);
  const contentHeight = Math.max(Number.EPSILON, maxY - minY);
  const viewWidth = Math.max(1, options.width - options.padding * 2);
  const viewHeight = Math.max(1, options.height - options.padding * 2);
  const scale = Math.min(viewWidth / contentWidth, viewHeight / contentHeight);

  const offsetX = options.padding + (viewWidth - contentWidth * scale) * 0.5;
  const offsetY = options.padding + (viewHeight - contentHeight * scale) * 0.5;

  return points.map((point) => ({
    ...point,
    groundX: offsetX + (point.groundX - minX) * scale,
    groundY: offsetY + (point.groundY - minY) * scale,
    topX: offsetX + (point.topX - minX) * scale,
    topY: offsetY + (point.topY - minY) * scale,
  }));
}

function revealPoints(points: PreparedPoint[], progress: number): PreparedPoint[] {
  if (points.length <= 1 || progress >= 1) return points;
  if (progress <= 0) return [points[0]];

  const out: PreparedPoint[] = [points[0]];
  for (let i = 0; i < points.length - 1; i += 1) {
    const current = points[i];
    const next = points[i + 1];
    if (next.routeProgress <= current.routeProgress) continue;
    if (next.routeProgress < progress) {
      out.push(next);
      continue;
    }
    const t = clamp(
      (progress - current.routeProgress) / (next.routeProgress - current.routeProgress),
      0,
      1,
    );
    out.push(lerpPoint(current, next, t));
    return out;
  }

  return points;
}

function lerpPoint(a: PreparedPoint, b: PreparedPoint, t: number): PreparedPoint {
  const lerp = (x: number, y: number) => x + (y - x) * t;
  return {
    groundX: lerp(a.groundX, b.groundX),
    groundY: lerp(a.groundY, b.groundY),
    topX: lerp(a.topX, b.topX),
    topY: lerp(a.topY, b.topY),
    value:
      a.value == null && b.value == null
        ? null
        : lerp(a.value ?? b.value ?? 0, b.value ?? a.value ?? 0),
    routeProgress: lerp(a.routeProgress, b.routeProgress),
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
