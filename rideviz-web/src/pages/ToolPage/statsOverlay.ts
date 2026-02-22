import type { AvailableData, Metrics, StatKey, VizData } from '../../types/api';
import type { StatsEntry } from '../../engine/types';

type TelemetrySample = {
  distanceKm: number;
  elevationGainM: number;
  elapsedSeconds: number | null;
  avgHeartRate: number | null;
  maxHeartRate: number | null;
  avgPower: number | null;
  maxPower: number | null;
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function formatDuration(totalSeconds: number): string {
  const seconds = Math.max(0, Math.round(totalSeconds));
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return h > 0
    ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
    : `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}

function interpolateOptional(a: number | null, b: number | null, t: number): number | null {
  if (a == null && b == null) return null;
  if (a == null) return b;
  if (b == null) return a;
  return a + (b - a) * t;
}

function fallbackTelemetry(metrics: Metrics): TelemetrySample {
  return {
    distanceKm: metrics.distance_km,
    elevationGainM: metrics.elevation_gain_m,
    elapsedSeconds: metrics.duration_seconds > 0 ? metrics.duration_seconds : null,
    avgHeartRate: metrics.avg_heart_rate,
    maxHeartRate: metrics.max_heart_rate,
    avgPower: metrics.avg_power,
    maxPower: metrics.max_power,
  };
}

function sampleTelemetry(routeData: VizData | null, progress: number): TelemetrySample | null {
  if (!routeData || routeData.points.length === 0) return null;
  const points = routeData.points;
  const clampedProgress = clamp(progress, 0, 1);
  if (clampedProgress <= 0) {
    const first = points[0];
    return {
      distanceKm: first.cumulative_distance_km,
      elevationGainM: first.cumulative_elevation_gain_m,
      elapsedSeconds: first.elapsed_seconds,
      avgHeartRate: first.cumulative_avg_heart_rate,
      maxHeartRate: first.cumulative_max_heart_rate,
      avgPower: first.cumulative_avg_power,
      maxPower: first.cumulative_max_power,
    };
  }
  if (clampedProgress >= 1) {
    const last = points[points.length - 1];
    return {
      distanceKm: last.cumulative_distance_km,
      elevationGainM: last.cumulative_elevation_gain_m,
      elapsedSeconds: last.elapsed_seconds,
      avgHeartRate: last.cumulative_avg_heart_rate,
      maxHeartRate: last.cumulative_max_heart_rate,
      avgPower: last.cumulative_avg_power,
      maxPower: last.cumulative_max_power,
    };
  }

  for (let idx = 0; idx < points.length - 1; idx += 1) {
    const current = points[idx];
    const next = points[idx + 1];
    if (next.route_progress <= current.route_progress) continue;
    if (next.route_progress < clampedProgress) continue;
    const t = clamp(
      (clampedProgress - current.route_progress) /
        (next.route_progress - current.route_progress),
      0,
      1,
    );
    return {
      distanceKm:
        current.cumulative_distance_km +
        (next.cumulative_distance_km - current.cumulative_distance_km) * t,
      elevationGainM:
        current.cumulative_elevation_gain_m +
        (next.cumulative_elevation_gain_m - current.cumulative_elevation_gain_m) * t,
      elapsedSeconds: interpolateOptional(current.elapsed_seconds, next.elapsed_seconds, t),
      avgHeartRate: interpolateOptional(
        current.cumulative_avg_heart_rate,
        next.cumulative_avg_heart_rate,
        t,
      ),
      maxHeartRate: interpolateOptional(
        current.cumulative_max_heart_rate,
        next.cumulative_max_heart_rate,
        t,
      ),
      avgPower: interpolateOptional(current.cumulative_avg_power, next.cumulative_avg_power, t),
      maxPower: interpolateOptional(current.cumulative_max_power, next.cumulative_max_power, t),
    };
  }

  const last = points[points.length - 1];
  return {
    distanceKm: last.cumulative_distance_km,
    elevationGainM: last.cumulative_elevation_gain_m,
    elapsedSeconds: last.elapsed_seconds,
    avgHeartRate: last.cumulative_avg_heart_rate,
    maxHeartRate: last.cumulative_max_heart_rate,
    avgPower: last.cumulative_avg_power,
    maxPower: last.cumulative_max_power,
  };
}

export function buildStatsEntries(
  keys: StatKey[],
  metrics: Metrics | null,
  availableData: AvailableData | null,
  routeData: VizData | null,
  progress: number,
): StatsEntry[] {
  if (!metrics || !availableData || keys.length === 0) return [];
  const telemetry = sampleTelemetry(routeData, progress) ?? fallbackTelemetry(metrics);
  const entries: StatsEntry[] = [];
  for (const key of keys) {
    if (key === 'distance') {
      entries.push({ key, label: 'DIST', value: `${telemetry.distanceKm.toFixed(1)} km` });
    }
    if (key === 'duration' && metrics.duration_seconds > 0 && telemetry.elapsedSeconds != null) {
      entries.push({ key, label: 'DUR', value: formatDuration(telemetry.elapsedSeconds) });
    }
    if (key === 'elevation_gain' && availableData.has_elevation) {
      entries.push({
        key,
        label: 'GAIN',
        value: `${Math.round(Math.max(0, telemetry.elevationGainM))} m`,
      });
    }
    if (key === 'avg_speed' && metrics.duration_seconds > 0 && telemetry.elapsedSeconds != null) {
      const speed =
        telemetry.elapsedSeconds > Number.EPSILON
          ? (telemetry.distanceKm / telemetry.elapsedSeconds) * 3600
          : 0;
      entries.push({ key, label: 'AVG SPD', value: `${Math.max(0, speed).toFixed(1)} km/h` });
    }
    if (key === 'avg_heart_rate' && availableData.has_heart_rate && telemetry.avgHeartRate != null) {
      entries.push({
        key,
        label: 'AVG HR',
        value: `${Math.round(Math.max(0, telemetry.avgHeartRate))} bpm`,
      });
    }
    if (key === 'max_heart_rate' && availableData.has_heart_rate && telemetry.maxHeartRate != null) {
      entries.push({
        key,
        label: 'MAX HR',
        value: `${Math.round(Math.max(0, telemetry.maxHeartRate))} bpm`,
      });
    }
    if (key === 'avg_power' && availableData.has_power && telemetry.avgPower != null) {
      entries.push({
        key,
        label: 'AVG PWR',
        value: `${Math.round(Math.max(0, telemetry.avgPower))} W`,
      });
    }
    if (key === 'max_power' && availableData.has_power && telemetry.maxPower != null) {
      entries.push({
        key,
        label: 'MAX PWR',
        value: `${Math.round(Math.max(0, telemetry.maxPower))} W`,
      });
    }
  }
  return entries;
}
