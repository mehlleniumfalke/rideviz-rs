import type { AvailableData, Metrics, StatKey } from '../../types/api';

interface StatsPickerProps {
  value: StatKey[];
  availableData: AvailableData;
  metrics: Metrics;
  onChange: (value: StatKey[]) => void;
}

type StatOption = {
  key: StatKey;
  label: string;
  available: (availableData: AvailableData, metrics: Metrics) => boolean;
};

const STAT_OPTIONS: StatOption[] = [
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

export default function StatsPicker({
  value,
  availableData,
  metrics,
  onChange,
}: StatsPickerProps) {
  const toggle = (stat: StatKey, enabled: boolean) => {
    if (enabled) {
      if (!value.includes(stat)) {
        onChange([...value, stat]);
      }
      return;
    }
    onChange(value.filter((entry) => entry !== stat));
  };

  return (
    <div className="box">
      <div className="label">Stats Overlay</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
        {STAT_OPTIONS.map((option) => {
          const enabled = isStatAvailable(option.key, availableData, metrics);
          const checked = value.includes(option.key);
          return (
            <label
              key={option.key}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 'var(--space-2)',
                fontSize: 'var(--text-xs)',
                opacity: enabled ? 1 : 0.5,
                cursor: enabled ? 'pointer' : 'not-allowed',
              }}
            >
              <input
                type="checkbox"
                checked={checked}
                disabled={!enabled}
                onChange={(event) => toggle(option.key, event.target.checked)}
              />
              <span>{option.label}</span>
            </label>
          );
        })}
      </div>
    </div>
  );
}
