import type { ExportPreset } from '../../types/api';
import { EXPORT_FORMAT_OPTIONS } from './exportFormats';

interface ExportFormatPickerProps {
  value: ExportPreset;
  onChange: (value: ExportPreset) => void;
}

export default function ExportFormatPicker({ value, onChange }: ExportFormatPickerProps) {
  return (
    <div className="box">
      <div className="label">Export format</div>
      <select
        value={value}
        onChange={(event) => onChange(event.target.value as ExportPreset)}
        style={{
          width: '100%',
          padding: 'var(--space-2)',
          border: 'var(--border)',
          borderRadius: 'var(--radius)',
          background: 'var(--white)',
          fontSize: 'var(--text-xs)',
        }}
      >
        {EXPORT_FORMAT_OPTIONS.map((format) => (
          <option key={format.value} value={format.value}>
            {format.label}
          </option>
        ))}
      </select>
    </div>
  );
}
