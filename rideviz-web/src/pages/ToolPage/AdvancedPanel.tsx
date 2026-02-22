interface AdvancedPanelProps {
  smoothing: number;
  glow: boolean;
  animated: boolean;
  hasProAccess: boolean;
  onChange: (config: {
    smoothing?: number;
    glow?: boolean;
    animated?: boolean;
  }) => void;
}

export default function AdvancedPanel({
  smoothing,
  glow,
  animated,
  hasProAccess,
  onChange,
}: AdvancedPanelProps) {
  return (
    <div className="box">
      <div className="label">Controls</div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
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

        <label style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-2)', cursor: 'pointer' }}>
          <input
            type="checkbox"
            checked={animated}
            onChange={(e) => onChange({ animated: e.target.checked })}
          />
          <span style={{ fontSize: 'var(--text-xs)' }}>Animated export (Pro)</span>
        </label>
        {!hasProAccess && (
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>
            Requires Pro license to export MP4.
          </span>
        )}

      </div>
    </div>
  );
}
