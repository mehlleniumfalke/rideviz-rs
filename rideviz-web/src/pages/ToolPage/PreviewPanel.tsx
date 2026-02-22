import type { RefObject } from 'react';
import type { BackgroundColor } from '../../types/api';

interface PreviewPanelProps {
  canvasRef: RefObject<HTMLCanvasElement | null>;
  previewWidth: number;
  previewHeight: number;
  isLoading: boolean;
  error: string | null;
  isExporting: boolean;
  onDownload: () => void;
  onShare: () => void;
  fileId: string | null;
  background: BackgroundColor;
  canShare: boolean;
  isAnimated: boolean;
  canAnimatedExport: boolean;
  shareStatus: string | null;
}

export default function PreviewPanel({
  canvasRef,
  previewWidth,
  previewHeight,
  isLoading,
  error,
  isExporting,
  onDownload,
  onShare,
  fileId,
  background,
  canShare,
  isAnimated,
  canAnimatedExport,
  shareStatus,
}: PreviewPanelProps) {
  const previewRatio = previewWidth / previewHeight;
  const previewAspectRatio = `${previewWidth} / ${previewHeight}`;
  const previewSurfaceStyle =
    background === 'transparent'
      ? {
          backgroundColor: '#fff',
          backgroundImage: 'repeating-conic-gradient(#e8e8e8 0% 25%, #ffffff 0% 50%)',
          backgroundSize: '16px 16px',
        }
      : { background: background === 'white' ? '#fff' : '#000' };

  return (
    <div
      className="preview-panel"
      style={{
        border: 'var(--border)',
        display: 'flex',
        flexDirection: 'column',
        minHeight: '500px',
      }}
    >
      {/* Preview area - animation is the centerpiece */}
      <div
        style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: 'var(--space-4)',
          background: '#fafafa',
        }}
      >
        {!fileId ? (
          <div style={{ textAlign: 'center', color: 'var(--gray)' }}>
            <div style={{ fontSize: '48px', marginBottom: 'var(--space-4)', opacity: 0.3 }}>↑</div>
            <div>Upload a file to preview</div>
          </div>
        ) : isLoading ? (
          <div style={{ color: 'var(--gray)' }} aria-live="polite">
            Loading...
          </div>
        ) : error ? (
          <div style={{ textAlign: 'center', color: '#c00' }} aria-live="polite">
            <div style={{ marginBottom: 'var(--space-2)' }}>⚠</div>
            <div style={{ fontSize: 'var(--text-sm)' }}>{error}</div>
          </div>
        ) : (
          <div
            style={{
              border: 'var(--border)',
              width: `min(960px, 100%, calc(60vh * ${previewRatio}))`,
              aspectRatio: previewAspectRatio,
              ...previewSurfaceStyle,
            }}
          >
            <canvas
              ref={canvasRef}
              aria-label="Route animation preview canvas"
              style={{
                display: 'block',
                width: '100%',
                height: '100%',
              }}
            />
          </div>
        )}
      </div>

      {/* Action buttons */}
      {fileId && !isLoading && !error && (
        <div style={{ padding: 'var(--space-4)', borderTop: 'var(--border)' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-2)' }}>
            <button
              onClick={onDownload}
              aria-label="Download generated export"
              disabled={isExporting || (isAnimated && !canAnimatedExport)}
              style={{ width: '100%', padding: 'var(--space-3)' }}
            >
              {isExporting ? 'Preparing...' : isAnimated ? (canAnimatedExport ? 'Export MP4 ↓' : 'Upgrade for MP4') : 'Download PNG ↓'}
            </button>
            <button onClick={onShare} aria-label="Share generated export" disabled={isExporting} style={{ width: '100%', padding: 'var(--space-3)' }}>
              {canShare ? 'Share ↗' : 'Share Link ↗'}
            </button>
          </div>
          {shareStatus && <div style={{ marginTop: 'var(--space-2)', fontSize: 'var(--text-xs)', color: 'var(--gray)' }} aria-live="polite">{shareStatus}</div>}
        </div>
      )}
    </div>
  );
}
