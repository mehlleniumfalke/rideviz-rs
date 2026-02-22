import type { RoutePoint } from '../types/api';

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function mapLinearProgressToRoute(points: RoutePoint[], linearProgress: number): number {
  const linear = clamp(linearProgress, 0, 1);
  if (points.length < 2) return linear;

  const timed = points.filter(
    (point): point is RoutePoint & { elapsed_seconds: number } =>
      typeof point.elapsed_seconds === 'number',
  );
  if (timed.length < 2) return linear;

  const totalElapsed = timed[timed.length - 1].elapsed_seconds;
  if (totalElapsed <= Number.EPSILON) return linear;

  const targetElapsed = linear * totalElapsed;
  if (targetElapsed <= timed[0].elapsed_seconds) {
    return clamp(timed[0].route_progress, 0, 1);
  }

  for (let idx = 1; idx < timed.length; idx += 1) {
    const prev = timed[idx - 1];
    const curr = timed[idx];
    const delta = curr.elapsed_seconds - prev.elapsed_seconds;
    if (delta <= Number.EPSILON) continue;
    if (targetElapsed > curr.elapsed_seconds) continue;
    const t = clamp((targetElapsed - prev.elapsed_seconds) / delta, 0, 1);
    return clamp(
      prev.route_progress + (curr.route_progress - prev.route_progress) * t,
      0,
      1,
    );
  }

  return clamp(timed[timed.length - 1].route_progress, 0, 1);
}
