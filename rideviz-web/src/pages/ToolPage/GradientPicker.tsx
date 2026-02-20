import type { GradientName } from '../../types/api';

interface GradientPickerProps {
  selectedGradient: GradientName;
  onChange: (gradient: GradientName) => void;
}

const GRADIENTS: { name: GradientName; colors: string[] }[] = [
  { name: 'rideviz', colors: ['#00c2ff', '#00e5a0', '#00ff94'] },
  { name: 'fire', colors: ['#ff2d55', '#ff6b35', '#ffb347'] },
  { name: 'ocean', colors: ['#0066ff', '#00b4d8', '#00f5d4'] },
  { name: 'sunset', colors: ['#ff6b6b', '#ffa07a', '#ffd93d'] },
  { name: 'forest', colors: ['#087f5b', '#20c997', '#96f2d7'] },
  { name: 'violet', colors: ['#7c3aed', '#a855f7', '#e879f9'] },
  { name: 'white', colors: ['#e8e4de', '#ffffff', '#e8e4de'] },
  { name: 'black', colors: ['#1a1a1a', '#0a0a0b', '#000000'] },
];

export default function GradientPicker({ selectedGradient, onChange }: GradientPickerProps) {
  return (
    <div className="box">
      <div className="label">Gradient</div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 'var(--space-2)' }}>
        {GRADIENTS.map((g) => (
          <button
            key={g.name}
            onClick={() => onChange(g.name)}
            style={{
              width: '100%',
              aspectRatio: '1',
              padding: 0,
              border: selectedGradient === g.name ? '2px solid var(--black)' : 'var(--border)',
              background: `linear-gradient(135deg, ${g.colors.join(', ')})`,
            }}
            title={g.name}
          />
        ))}
      </div>
      <div style={{ marginTop: 'var(--space-2)', fontSize: 'var(--text-xs)', color: 'var(--gray)', textAlign: 'center' }}>
        {selectedGradient}
      </div>
    </div>
  );
}
