import { useRef, useState } from 'react';

interface UploadZoneProps {
  onFileSelect: (file: File) => void;
  isUploading: boolean;
  error?: string;
}

export default function UploadZone({ onFileSelect, isUploading, error }: UploadZoneProps) {
  const [dragActive, setDragActive] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

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

  return (
    <div
      onDragEnter={handleDrag}
      onDragLeave={handleDrag}
      onDragOver={handleDrag}
      onDrop={handleDrop}
      onClick={() => inputRef.current?.click()}
      style={{
        border: dragActive ? '2px solid var(--black)' : '1px dashed var(--black)',
        padding: 'var(--space-8)',
        textAlign: 'center',
        cursor: 'pointer',
        background: dragActive ? '#f5f5f5' : 'var(--white)',
      }}
    >
      <input
        ref={inputRef}
        type="file"
        accept=".gpx,.fit"
        onChange={handleChange}
        className="visually-hidden"
      />

      {isUploading ? (
        <div>Uploading...</div>
      ) : (
        <>
          <div style={{ fontSize: 'var(--text-lg)', marginBottom: 'var(--space-2)' }}>
            Drop GPX or FIT file
          </div>
          <div style={{ fontSize: 'var(--text-sm)', color: 'var(--gray)' }}>
            or click to browse
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
