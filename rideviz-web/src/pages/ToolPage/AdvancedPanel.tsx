interface AdvancedPanelProps {
  strokeWidth: number;
  padding: number;
  smoothing: number;
  glow: boolean;
  animated: boolean;
  onChange: (config: {
    strokeWidth?: number;
    padding?: number;
    smoothing?: number;
    glow?: boolean;
    animated?: boolean;
  }) => void;
}

export default function AdvancedPanel({
  strokeWidth,
  padding,
  smoothing,
  glow,
  animated,
  onChange,
}: AdvancedPanelProps) {
  return (
    <div className="box">
      <div className="label">Controls</div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
        {/* Stroke Width */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-1)' }}>
            <span style={{ fontSize: 'var(--text-xs)' }}>Stroke Width</span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>{strokeWidth}</span>
          </div>
          <input
            type="range"
            min={1}
            max={12}
            value={strokeWidth}
            onChange={(e) => onChange({ strokeWidth: Number(e.target.value) })}
          />
        </div>

        {/* Padding */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-1)' }}>
            <span style={{ fontSize: 'var(--text-xs)' }}>Padding</span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>{padding}</span>
          </div>
          <input
            type="range"
            min={0}
            max={120}
            value={padding}
            onChange={(e) => onChange({ padding: Number(e.target.value) })}
          />
        </div>

        {/* Smoothing */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-1)' }}>
            <span style={{ fontSize: 'var(--text-xs)' }}>Smoothing</span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>{smoothing}</span>
          </div>
          <input
            type="range"
            min={0}
            max={100}
            value={smoothing}
            onChange={(e) => onChange({ smoothing: Number(e.target.value) })}
          />
        </div>

        {/* Glow */}
        <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', cursor: 'pointer' }}>
          <input
            type="checkbox"
            checked={glow}
            onChange={(e) => onChange({ glow: e.target.checked })}
          />
          <span style={{ fontSize: 'var(--text-xs)' }}>Enable glow effect</span>
        </label>

        {/* Animated */}
        <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', cursor: 'pointer' }}>
          <input
            type="checkbox"
            checked={animated}
            onChange={(e) => onChange({ animated: e.target.checked })}
          />
          <span style={{ fontSize: 'var(--text-xs)' }}>3D animation</span>
        </label>
      </div>
    </div>
  );
}
