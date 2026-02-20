# RideViz Frontend Integration (Route-Only)

This document defines the current frontend/backend contract for the **route 3D animation** product surface.

## Scope

- Upload GPX/FIT
- Configure route style controls shown in the UI
- Generate APNG output for preview/download
- No legacy non-route visualization modes

## API Contract

### Upload

`POST /api/upload` with multipart `file`.

Response:

```ts
interface UploadResponse {
  file_id: string;
  file_type: 'gpx' | 'fit';
  metrics: Metrics;
  available_data: AvailableData;
}

interface AvailableData {
  has_coordinates: boolean;
  has_elevation: boolean;
  has_heart_rate: boolean;
  has_power: boolean;
}

interface Metrics {
  distance_km: number;
  elevation_gain_m: number;
  duration_seconds: number;
  avg_speed_kmh: number;
  avg_heart_rate: number | null;
  max_heart_rate: number | null;
  avg_power: number | null;
  max_power: number | null;
}
```

### Visualize

`POST /api/visualize` with JSON:

```ts
type ColorByMetric = 'elevation' | 'speed' | 'heartrate' | 'power';
type GradientName =
  | 'fire'
  | 'ocean'
  | 'sunset'
  | 'forest'
  | 'violet'
  | 'rideviz'
  | 'white'
  | 'black';

interface VisualizeRequest {
  file_id: string;
  gradient: GradientName;
  color_by?: ColorByMetric;
  stroke_width?: number;
  padding?: number;
  smoothing?: number;
  glow?: boolean;
  animation_frames?: number;
  animation_duration_ms?: number;
}
```

Response: binary `image/apng`.

## Backend-fixed behavior

The backend is intentionally fixed to current product defaults:

- route visualization only
- 3D elevation extrusion
- animated APNG output
- `1920x1080` rendering surface
- transparent background

## Frontend notes

- If `available_data.has_elevation` is false, show an error state for 3D rendering.
- `color_by` can still target `heartrate` and `power` when data exists.
- Preview and download can use different frame/duration profiles.
