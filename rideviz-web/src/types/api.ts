export interface UploadResponse {
  file_id: string;
  file_type: 'gpx' | 'fit' | 'strava';
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

export type ExportPreset =
  | 'story_9x16'
  | 'instagram_post_portrait_4x5'
  | 'instagram_post_square_1x1'
  | 'x_post_16x9'
  | 'facebook_feed_landscape'
  | 'facebook_feed_square'
  | 'hd_landscape_16x9';
export type ColorByMetric = 'elevation' | 'speed' | 'heartrate' | 'power';
export type BackgroundColor = 'transparent' | 'white' | 'black';
export type OutputFormat = 'png';
export type StatKey =
  | 'distance'
  | 'duration'
  | 'elevation_gain'
  | 'avg_speed'
  | 'avg_heart_rate'
  | 'max_heart_rate'
  | 'avg_power'
  | 'max_power';
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
  width?: number;
  height?: number;
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
  stats?: StatKey[];
}

export interface VideoExportRequest {
  file_id: string;
  gradient: GradientName;
  width?: number;
  height?: number;
  stroke_width?: number;
  padding?: number;
  smoothing?: number;
  color_by?: ColorByMetric;
  glow?: boolean;
  background?: Exclude<BackgroundColor, 'transparent'>;
  duration_seconds: number;
  fps: number;
  stats?: StatKey[];
}

export interface RoutePoint {
  x: number;
  y: number;
  value: number | null;
  elevation: number | null;
  route_progress: number;
  cumulative_distance_km: number;
  cumulative_elevation_gain_m: number;
  elapsed_seconds: number | null;
  heart_rate: number | null;
  power: number | null;
  cumulative_avg_heart_rate: number | null;
  cumulative_max_heart_rate: number | null;
  cumulative_avg_power: number | null;
  cumulative_max_power: number | null;
}

export interface VizData {
  points: RoutePoint[];
}

export interface RouteDataResponse {
  file_id: string;
  viz_data: VizData;
  metrics: Metrics;
  available_data: AvailableData;
}

export interface StravaAuthResponse {
  auth_url: string;
  state: string;
}

export interface StravaByoCredentials {
  client_id: string;
  client_secret: string;
}

export interface StravaCallbackResponse {
  access_token: string;
  athlete_id: number | null;
  expires_in_seconds: number;
}

export interface StravaActivitySummary {
  id: number;
  name: string;
  distance_m: number;
  start_date: string | null;
}

export interface StravaActivitiesResponse {
  activities: StravaActivitySummary[];
  next_page: number | null;
}

export interface CheckoutResponse {
  checkout_url: string;
  mode: 'live' | 'mock';
}

export interface LicenseResponse {
  token: string;
  pro: boolean;
  expires_in_seconds: number;
}

export interface LicenseVerifyResponse {
  valid: boolean;
  pro: boolean;
  email: string;
}
