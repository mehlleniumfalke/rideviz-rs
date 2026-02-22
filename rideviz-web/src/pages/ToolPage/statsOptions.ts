import type { AvailableData, Metrics, StatKey } from '../../types/api';

type StatOption = {
  key: StatKey;
  label: string;
  available: (availableData: AvailableData, metrics: Metrics) => boolean;
};

export const STAT_OPTIONS: StatOption[] = [
  {
    key: 'distance',
    label: 'Distance',
    available: () => true,
  },
  {
    key: 'duration',
    label: 'Duration',
    available: (_availableData, metrics) => metrics.duration_seconds > 0,
  },
  {
    key: 'elevation_gain',
    label: 'Elevation Gain',
    available: (availableData) => availableData.has_elevation,
  },
  {
    key: 'avg_speed',
    label: 'Avg Speed',
    available: (_availableData, metrics) => metrics.duration_seconds > 0,
  },
  {
    key: 'avg_heart_rate',
    label: 'Avg Heart Rate',
    available: (availableData, metrics) =>
      availableData.has_heart_rate && metrics.avg_heart_rate !== null,
  },
  {
    key: 'max_heart_rate',
    label: 'Max Heart Rate',
    available: (availableData, metrics) =>
      availableData.has_heart_rate && metrics.max_heart_rate !== null,
  },
  {
    key: 'avg_power',
    label: 'Avg Power',
    available: (availableData, metrics) => availableData.has_power && metrics.avg_power !== null,
  },
  {
    key: 'max_power',
    label: 'Max Power',
    available: (availableData, metrics) => availableData.has_power && metrics.max_power !== null,
  },
];

export function isStatAvailable(
  stat: StatKey,
  availableData: AvailableData,
  metrics: Metrics,
): boolean {
  const option = STAT_OPTIONS.find((entry) => entry.key === stat);
  return option ? option.available(availableData, metrics) : false;
}
