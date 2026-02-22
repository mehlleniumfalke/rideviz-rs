import type { GradientName, BackgroundColor, ExportPreset, ColorByMetric, StatKey } from '../types/api';

const HISTORY_KEY = 'rideviz_recent_generations_v1';
const MAX_ITEMS = 10;

export interface StoredGenerationConfig {
  gradient: GradientName;
  exportPreset: ExportPreset;
  colorBy: ColorByMetric | null;
  strokeWidth: number;
  padding: number;
  smoothing: number;
  glow: boolean;
  background: BackgroundColor;
  animated: boolean;
  duration: number;
  fps: number;
  stats: StatKey[];
}

export interface StoredGeneration {
  id: string;
  createdAt: string;
  thumbnailDataUrl: string;
  config: StoredGenerationConfig;
}

export function loadGenerationHistory(): StoredGeneration[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as StoredGeneration[];
    if (!Array.isArray(parsed)) return [];
    return parsed.slice(0, MAX_ITEMS);
  } catch {
    return [];
  }
}

export function saveGenerationHistory(entries: StoredGeneration[]): void {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(entries.slice(0, MAX_ITEMS)));
}

export function addGenerationHistoryEntry(entry: StoredGeneration): StoredGeneration[] {
  const existing = loadGenerationHistory().filter((item) => item.id !== entry.id);
  const next = [entry, ...existing].slice(0, MAX_ITEMS);
  saveGenerationHistory(next);
  return next;
}
