import { useState } from 'react';
import type { AvailableData, Metrics, StatKey } from '../../types/api';
import { isStatAvailable, STAT_OPTIONS } from './statsOptions';

interface StatsPickerProps {
  value: StatKey[];
  availableData: AvailableData;
  metrics: Metrics;
  onChange: (value: StatKey[]) => void;
}

export default function StatsPicker({
  value,
  availableData,
  metrics,
  onChange,
}: StatsPickerProps) {
  const [isOpen, setIsOpen] = useState(false);

  const toggle = (stat: StatKey, enabled: boolean) => {
    if (enabled) {
      if (!value.includes(stat)) {
        onChange([...value, stat]);
      }
      return;
    }
    onChange(value.filter((entry) => entry !== stat));
  };

  return (
    <div className="box">
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        style={{
          all: 'unset',
          display: 'flex',
          width: '100%',
          justifyContent: 'space-between',
          alignItems: 'center',
          cursor: 'pointer',
          fontSize: 'var(--text-xs)',
          fontWeight: 600,
          marginBottom: isOpen ? 'var(--space-2)' : 0,
        }}
        aria-expanded={isOpen}
        aria-label="Toggle stats overlay options"
      >
        <span className="label" style={{ margin: 0 }}>Stats Overlay</span>
        <span aria-hidden>{isOpen ? 'v' : '>'}</span>
      </button>
      {isOpen && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
          {STAT_OPTIONS.map((option) => {
            const enabled = isStatAvailable(option.key, availableData, metrics);
            const checked = value.includes(option.key);
            return (
              <label
                key={option.key}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 'var(--space-2)',
                  fontSize: 'var(--text-xs)',
                  opacity: enabled ? 1 : 0.5,
                  cursor: enabled ? 'pointer' : 'not-allowed',
                }}
              >
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={!enabled}
                  onChange={(event) => toggle(option.key, event.target.checked)}
                />
                <span>{option.label}</span>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}

