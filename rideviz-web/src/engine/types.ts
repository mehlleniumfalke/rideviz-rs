import type { BackgroundColor, GradientName, StatKey, VizData } from '../types/api';

export interface RenderFrameOptions {
  width: number;
  height: number;
  padding: number;
  strokeWidth: number;
  smoothing: number;
  glow: boolean;
  background: BackgroundColor;
  gradient: GradientName;
  progress: number;
}

export interface StatsEntry {
  key: StatKey;
  label: string;
  value: string;
}

export interface FrameRenderInput {
  data: VizData;
  options: RenderFrameOptions;
  stats: StatsEntry[];
}

export interface PreparedPoint {
  groundX: number;
  groundY: number;
  topX: number;
  topY: number;
  value: number | null;
  routeProgress: number;
}
