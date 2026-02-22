import { useEffect, useRef, useState } from 'react';

interface UploadZoneProps {
  onFileSelect: (file: File) => void;
  isUploading: boolean;
  error?: string;
}

export default function UploadZone({ onFileSelect, isUploading, error }: UploadZoneProps) {
  const [dragActive, setDragActive] = useState(false);
  const [isTouchDevice, setIsTouchDevice] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const zoneId = 'file-upload-zone';

  useEffect(() => {
    const media = window.matchMedia('(pointer: coarse)');
    const sync = () => setIsTouchDevice(media.matches);
    sync();
    media.addEventListener('change', sync);
    return () => media.removeEventListener('change', sync);
  }, []);

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === 'dragenter' || e.type === 'dragover') {
      setDragActive(true);
    } else if (e.type === 'dragleave') {
      setDragActive(false);
    }
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);

    const file = e.dataTransfer.files?.[0];
    if (file && (file.name.endsWith('.gpx') || file.name.endsWith('.fit'))) {
      onFileSelect(file);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      onFileSelect(file);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      inputRef.current?.click();
    }
  };

  return (
    <div
      onDragEnter={handleDrag}
      onDragLeave={handleDrag}
      onDragOver={handleDrag}
      onDrop={handleDrop}
      onClick={() => inputRef.current?.click()}
      onKeyDown={handleKeyDown}
      role="button"
      tabIndex={0}
      aria-label="Upload GPX or FIT file"
      aria-describedby={zoneId}
      style={{
        border: dragActive ? '2px solid var(--black)' : '1px dashed var(--black)',
        padding: 'var(--space-8)',
        minHeight: 180,
        textAlign: 'center',
        cursor: 'pointer',
        background: dragActive ? '#f5f5f5' : 'var(--white)',
      }}
    >
      <input
        ref={inputRef}
        id="file-upload-input"
        type="file"
        accept=".gpx,.fit"
        onChange={handleChange}
        className="visually-hidden"
      />

      {isUploading ? (
        <div style={{ display: 'inline-flex', alignItems: 'center', gap: 'var(--space-2)' }}>
          <span className="spinner" aria-hidden />
          <span>Uploading file...</span>
        </div>
      ) : (
        <>
          <svg
            viewBox="0 0 24 24"
            width="36"
            height="36"
            aria-hidden
            style={{ opacity: 0.6, marginBottom: 'var(--space-3)' }}
          >
            <path
              d="M12 3 8.2 6.8a1 1 0 0 0 1.4 1.4l1.4-1.4V15a1 1 0 1 0 2 0V6.8l1.4 1.4a1 1 0 0 0 1.4-1.4L12 3Zm-7 13a1 1 0 0 0-1 1v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2a1 1 0 1 0-2 0v2H6v-2a1 1 0 0 0-1-1Z"
              fill="currentColor"
            />
          </svg>
          <div style={{ fontSize: 'var(--text-lg)', marginBottom: 'var(--space-2)' }}>
            {isTouchDevice ? 'Select GPX or FIT file' : 'Drop GPX or FIT file'}
          </div>
          <div id={zoneId} style={{ fontSize: 'var(--text-sm)', color: 'var(--gray)' }}>
            {isTouchDevice ? 'Tap to choose from files' : 'or click to browse'}
          </div>
        </>
      )}

      {error && (
        <div style={{ marginTop: 'var(--space-3)', fontSize: 'var(--text-sm)', color: '#c00' }}>
          {error}
        </div>
      )}
    </div>
  );
}
