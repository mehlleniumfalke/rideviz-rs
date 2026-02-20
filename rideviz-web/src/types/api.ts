export interface UploadResponse {
  file_id: string;
  file_type: 'gpx' | 'fit';
  metrics: Metrics;
  available_data: AvailableData;
}

export interface AvailableData {
  has_coordinates: boolean;
  has_elevation: boolean;
  has_heart_rate: boolean;
  has_power: boolean;
}

export interface Metrics {
  distance_km: number;
  elevation_gain_m: number;
  duration_seconds: number;
  avg_speed_kmh: number;
  avg_heart_rate: number | null;
  max_heart_rate: number | null;
  avg_power: number | null;
  max_power: number | null;
}

export type Format = 'story' | 'post' | 'wide' | 'custom';
export type ColorByMetric = 'elevation' | 'speed' | 'heartrate' | 'power';
export type BackgroundColor = 'transparent' | 'white' | 'black';
export type OutputFormat = 'apng' | 'webm';
export type GradientName =
  | 'fire'
  | 'ocean'
  | 'sunset'
  | 'forest'
  | 'violet'
  | 'rideviz'
  | 'white'
  | 'black';

export interface VisualizeRequest {
  file_id: string;
  gradient: GradientName;
  stroke_width?: number;
  padding?: number;
  smoothing?: number;
  color_by?: ColorByMetric;
  glow?: boolean;
  background?: BackgroundColor;
  duration_seconds?: number;
  fps?: number;
  animation_frames?: number;
  animation_duration_ms?: number;
  watermark?: boolean;
  format?: OutputFormat;
}
