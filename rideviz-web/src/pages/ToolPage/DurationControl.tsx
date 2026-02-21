interface DurationControlProps {
  duration: number;
  fps: number;
  width?: number;
  height?: number;
  onChange: (config: { duration?: number; fps?: number }) => void;
}

type Preset = {
  label: string;
  duration: number;
};

const presets: Preset[] = [
  { label: 'Story', duration: 6 },
  { label: 'Reel', duration: 15 },
  { label: 'YouTube', duration: 30 },
];

const snapPoints = [6, 9, 15, 30];

function estimateFileSizeKB(duration: number, fps: number, width = 1920, height = 1080): number {
  const frames = Math.round(duration * fps);
  const megapixels = (width * height) / 1_000_000;
  const bytesPerFrame = megapixels * 50_000;
  return Math.round((frames * bytesPerFrame) / 1024);
}

function formatFileSize(kb: number): string {
  if (kb < 1024) {
    return `${kb} KB`;
  }
  return `${(kb / 1024).toFixed(1)} MB`;
}

function findNearestSnapPoint(value: number): number {
  let nearest = value;
  let minDiff = Infinity;
  
  for (const snap of snapPoints) {
    const diff = Math.abs(value - snap);
    if (diff < minDiff && diff < 1.5) {
      minDiff = diff;
      nearest = snap;
    }
  }
  
  return nearest;
}

export default function DurationControl({
  duration,
  fps,
  width = 1920,
  height = 1080,
  onChange,
}: DurationControlProps) {
  const estimatedSize = estimateFileSizeKB(duration, fps, width, height);

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = Number(e.target.value);
    const snapped = findNearestSnapPoint(value);
    onChange({ duration: snapped });
  };

  return (
    <div className="box">
      <div className="label">Duration & Speed</div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
        {/* Platform Presets */}
        <div>
          <div style={{ fontSize: 'var(--text-xs)', marginBottom: 'var(--space-2)', color: 'var(--gray)' }}>
            Platform presets
          </div>
          <div style={{ display: 'flex', gap: 'var(--space-2)' }}>
            {presets.map((preset) => (
              <button
                key={preset.label}
                onClick={() => onChange({ duration: preset.duration })}
                style={{
                  flex: 1,
                  padding: 'var(--space-2)',
                  fontSize: 'var(--text-xs)',
                  border: 'var(--border)',
                  borderRadius: 'var(--radius)',
                  background: duration === preset.duration ? 'var(--bg-active)' : 'transparent',
                  cursor: 'pointer',
                  transition: 'all 0.15s',
                }}
              >
                {preset.label}
                <div style={{ fontSize: '10px', opacity: 0.6, marginTop: '2px' }}>
                  {preset.duration}s
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Duration Slider */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-1)' }}>
            <span style={{ fontSize: 'var(--text-xs)' }}>Duration</span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>{duration}s</span>
          </div>
          <input
            type="range"
            min={3}
            max={60}
            step={1}
            value={duration}
            onChange={handleSliderChange}
            style={{ width: '100%' }}
          />
          <div
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              fontSize: '10px',
              color: 'var(--gray)',
              marginTop: 'var(--space-1)',
            }}
          >
            <span>3s</span>
            <span>60s</span>
          </div>
        </div>

        {/* FPS Selector */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-2)' }}>
            <span style={{ fontSize: 'var(--text-xs)' }}>Frame rate</span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>{fps} fps</span>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 'var(--space-2)' }}>
            {[15, 24, 30, 60].map((fpsOption) => (
              <button
                key={fpsOption}
                onClick={() => onChange({ fps: fpsOption })}
                style={{
                  padding: 'var(--space-2)',
                  fontSize: 'var(--text-xs)',
                  border: 'var(--border)',
                  borderRadius: 'var(--radius)',
                  background: fps === fpsOption ? 'var(--bg-active)' : 'transparent',
                  cursor: 'pointer',
                  transition: 'all 0.15s',
                }}
              >
                {fpsOption}
              </button>
            ))}
          </div>
        </div>

        {/* File Size Estimate */}
        <div
          style={{
            padding: 'var(--space-2)',
            background: 'var(--bg-secondary)',
            borderRadius: 'var(--radius)',
            fontSize: 'var(--text-xs)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span style={{ color: 'var(--gray)' }}>Est. file size</span>
          <span style={{ fontWeight: 500 }}>{formatFileSize(estimatedSize)}</span>
        </div>
      </div>
    </div>
  );
}
