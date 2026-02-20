import type { BackgroundColor } from '../../types/api';

interface BackgroundPickerProps {
  value: BackgroundColor;
  onChange: (value: BackgroundColor) => void;
}

const BACKGROUNDS: { value: BackgroundColor; label: string }[] = [
  { value: 'transparent', label: 'Transparent' },
  { value: 'white', label: 'White' },
  { value: 'black', label: 'Black' },
];

export default function BackgroundPicker({ value, onChange }: BackgroundPickerProps) {
  return (
    <div className="box">
      <div className="label">Background</div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 'var(--space-2)' }}>
        {BACKGROUNDS.map((bg) => (
          <button
            key={bg.value}
            onClick={() => onChange(bg.value)}
            style={{
              border: value === bg.value ? '2px solid var(--black)' : 'var(--border)',
              background: value === bg.value ? '#f0f0f0' : 'var(--white)',
              padding: 'var(--space-2)',
              fontSize: 'var(--text-xs)',
            }}
          >
            {bg.label}
          </button>
        ))}
      </div>
    </div>
  );
}
