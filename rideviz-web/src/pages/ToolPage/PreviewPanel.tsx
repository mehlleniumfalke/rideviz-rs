import { useState, useEffect } from 'react';
import type { BackgroundColor } from '../../types/api';

interface PreviewPanelProps {
  previewUrl: string | null;
  isLoading: boolean;
  error: string | null;
  onDownload: () => void;
  fileId: string | null;
  background: BackgroundColor;
}

export default function PreviewPanel({
  previewUrl,
  isLoading,
  error,
  onDownload,
  fileId,
  background,
}: PreviewPanelProps) {
  const [imageLoaded, setImageLoaded] = useState(false);

  useEffect(() => {
    if (previewUrl) setImageLoaded(false);
  }, [previewUrl]);

  // Show white background for transparent images since page bg is white
  const previewBg = background === 'transparent' ? '#fff' : background === 'white' ? '#fff' : '#000';

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
          <div style={{ color: 'var(--gray)' }}>Loading...</div>
        ) : error ? (
          <div style={{ textAlign: 'center', color: '#c00' }}>
            <div style={{ marginBottom: 'var(--space-2)' }}>⚠</div>
            <div style={{ fontSize: 'var(--text-sm)' }}>{error}</div>
          </div>
        ) : previewUrl ? (
          <div
            style={{
              border: 'var(--border)',
              background: previewBg,
              maxWidth: '100%',
              maxHeight: '100%',
            }}
          >
            <img
              src={previewUrl}
              alt="Route animation preview"
              onLoad={() => setImageLoaded(true)}
              style={{
                display: 'block',
                maxWidth: '100%',
                maxHeight: '60vh',
                opacity: imageLoaded ? 1 : 0,
                transition: 'opacity 200ms',
              }}
            />
          </div>
        ) : null}
      </div>

      {/* Download button */}
      {previewUrl && !isLoading && !error && (
        <div style={{ padding: 'var(--space-4)', borderTop: 'var(--border)' }}>
          <button onClick={onDownload} style={{ width: '100%', padding: 'var(--space-3)' }}>
            Download Animation ↓
          </button>
        </div>
      )}
    </div>
  );
}
