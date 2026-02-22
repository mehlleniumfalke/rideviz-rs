import type { ColorByMetric, GradientName } from '../../types/api';

interface AvailableColorByData {
  has_elevation: boolean;
  has_heart_rate: boolean;
  has_power: boolean;
}

interface ColorByPickerProps {
  value: ColorByMetric | null;
  gradient: GradientName;
  availableData: AvailableColorByData;
  onChange: (value: ColorByMetric | null) => void;
}

type ColorByOption = {
  value: ColorByMetric | null;
  label: string;
  disabled: boolean;
};

export default function ColorByPicker({ value, gradient, availableData, onChange }: ColorByPickerProps) {
  const disableMetricColoring = gradient === 'black';
  const options: ColorByOption[] = [
    { value: null, label: 'Gradient', disabled: false },
    { value: 'elevation', label: 'Elevation', disabled: disableMetricColoring || !availableData.has_elevation },
    { value: 'speed', label: 'Speed', disabled: disableMetricColoring },
    { value: 'heartrate', label: 'Heart Rate', disabled: disableMetricColoring || !availableData.has_heart_rate },
    { value: 'power', label: 'Power', disabled: disableMetricColoring || !availableData.has_power },
  ];

  return (
    <div className="box">
      <div className="label">Route Coloring</div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 'var(--space-2)' }}>
        {options.map((option) => (
          <button
            key={option.label}
            onClick={() => !option.disabled && onChange(option.value)}
            disabled={option.disabled}
            style={{
              border: value === option.value ? '2px solid var(--black)' : 'var(--border)',
              background: value === option.value ? '#f0f0f0' : 'var(--white)',
              padding: 'var(--space-2)',
              fontSize: 'var(--text-xs)',
            }}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}
