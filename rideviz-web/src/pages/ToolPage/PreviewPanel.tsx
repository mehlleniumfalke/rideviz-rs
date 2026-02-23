import type { ReactNode, RefObject } from 'react';
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
  showWatermark?: boolean;
  emptyState?: ReactNode;
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
  showWatermark,
  emptyState,
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
          emptyState ?? (
            <div style={{ textAlign: 'center', color: 'var(--gray)' }}>
              <div style={{ fontSize: '48px', marginBottom: 'var(--space-4)', opacity: 0.3 }}>↑</div>
              <div>Upload a file to preview</div>
            </div>
          )
        ) : isLoading ? (
          <div style={{ color: 'var(--gray)', display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2)' }} aria-live="polite">
            <span className="spinner" aria-hidden />
            <span>Rendering preview...</span>
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
              position: 'relative',
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
            {showWatermark && !isAnimated && (
              <div
                aria-label="Watermark"
                style={{
                  position: 'absolute',
                  left: '50%',
                  bottom: '12px',
                  transform: 'translateX(-50%)',
                  padding: 'clamp(6px, 0.8vw, 10px) clamp(10px, 1.2vw, 16px)',
                  borderRadius: '999px',
                  background: 'rgba(0,0,0,0.35)',
                  border: '1px solid rgba(255,255,255,0.22)',
                  color: 'rgba(255,255,255,0.92)',
                  fontFamily: 'Geist Pixel, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
                  fontSize: 'clamp(12px, 1.6vw, 18px)',
                  lineHeight: 1,
                  letterSpacing: '0.2px',
                  userSelect: 'none',
                  pointerEvents: 'none',
                }}
              >
                created with rideviz.online
              </div>
            )}
          </div>
        )}
      </div>

      {/* Action buttons */}
      {fileId && !isLoading && !error && (
        <div className="preview-actions" style={{ padding: 'var(--space-4)', borderTop: 'var(--border)' }}>
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
          {isExporting && <div className="progress-indeterminate" style={{ marginTop: 'var(--space-2)' }} aria-hidden />}
          {shareStatus && <div style={{ marginTop: 'var(--space-2)', fontSize: 'var(--text-xs)', color: 'var(--gray)' }} aria-live="polite">{shareStatus}</div>}
        </div>
      )}
    </div>
  );
}
