import type { AvailableData, Metrics, StatKey } from '../../types/api';
import type { StatsEntry } from '../../engine/types';

export function buildStatsEntries(
  keys: StatKey[],
  metrics: Metrics | null,
  availableData: AvailableData | null,
): StatsEntry[] {
  if (!metrics || !availableData || keys.length === 0) return [];
  const entries: StatsEntry[] = [];
  for (const key of keys) {
    if (key === 'distance') entries.push({ key, label: 'DIST', value: `${metrics.distance_km.toFixed(1)} km` });
    if (key === 'duration' && metrics.duration_seconds > 0) {
      const total = metrics.duration_seconds;
      const h = Math.floor(total / 3600);
      const m = Math.floor((total % 3600) / 60);
      const s = total % 60;
      const value = h > 0 ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
      entries.push({ key, label: 'DUR', value });
    }
    if (key === 'elevation_gain' && availableData.has_elevation) entries.push({ key, label: 'GAIN', value: `${Math.round(metrics.elevation_gain_m)} m` });
    if (key === 'avg_speed' && metrics.duration_seconds > 0) entries.push({ key, label: 'AVG SPD', value: `${metrics.avg_speed_kmh.toFixed(1)} km/h` });
    if (key === 'avg_heart_rate' && availableData.has_heart_rate && metrics.avg_heart_rate !== null) entries.push({ key, label: 'AVG HR', value: `${metrics.avg_heart_rate} bpm` });
    if (key === 'max_heart_rate' && availableData.has_heart_rate && metrics.max_heart_rate !== null) entries.push({ key, label: 'MAX HR', value: `${metrics.max_heart_rate} bpm` });
    if (key === 'avg_power' && availableData.has_power && metrics.avg_power !== null) entries.push({ key, label: 'AVG PWR', value: `${metrics.avg_power} W` });
    if (key === 'max_power' && availableData.has_power && metrics.max_power !== null) entries.push({ key, label: 'MAX PWR', value: `${metrics.max_power} W` });
  }
  return entries;
}
