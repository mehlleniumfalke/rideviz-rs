interface DurationControlProps {
  duration: number;
  fps: number;
  width?: number;
  height?: number;
  onChange: (config: { duration?: number; fps?: number }) => void;
}

const snapPoints = [3, 6, 9, 12, 15];

const MAX_MP4_PIXELS_720P = 1280 * 720;
const MP4_ESTIMATED_BITS_PER_PIXEL = 0.07;
const MP4_ESTIMATED_OVERHEAD_MULTIPLIER = 1.05;

function capMp4DimensionsTo720p(width: number, height: number): [number, number] {
  const pixels = width * height;
  if (pixels <= MAX_MP4_PIXELS_720P) {
    return [width & ~1, height & ~1];
  }

  const scale = Math.sqrt(MAX_MP4_PIXELS_720P / pixels);
  let scaledWidth = Math.round(width * scale);
  let scaledHeight = Math.round(height * scale);

  if (scaledWidth % 2 !== 0) scaledWidth -= 1;
  if (scaledHeight % 2 !== 0) scaledHeight -= 1;

  return [Math.max(320, scaledWidth), Math.max(320, scaledHeight)];
}

function estimateFileSizeKB(duration: number, fps: number, width = 1920, height = 1080): number {
  const boundedDuration = Math.min(15, Math.max(3, duration));
  const boundedFps = Math.min(30, Math.max(24, fps));
  const [videoWidth, videoHeight] = capMp4DimensionsTo720p(width, height);
  const pixelsPerSecond = videoWidth * videoHeight * boundedFps;
  const estimatedBits = pixelsPerSecond * boundedDuration * MP4_ESTIMATED_BITS_PER_PIXEL;
  const estimatedBytes = (estimatedBits / 8) * MP4_ESTIMATED_OVERHEAD_MULTIPLIER;
  return Math.max(200, Math.round(estimatedBytes / 1024));
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
        {/* Duration Slider */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-1)' }}>
            <span style={{ fontSize: 'var(--text-xs)' }}>Duration</span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>{duration}s</span>
          </div>
          <input
            type="range"
            min={3}
            max={15}
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
            <span>15s</span>
          </div>
        </div>

        {/* FPS Selector */}
        <div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 'var(--space-2)' }}>
            <span style={{ fontSize: 'var(--text-xs)' }}>Frame rate</span>
            <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>{fps} fps</span>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 'var(--space-2)' }}>
            {[24, 30].map((fpsOption) => (
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
