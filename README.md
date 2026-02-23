# RideViz-RS

High-performance Rust backend for **RideViz**: turn GPX/FIT activities into route visuals.

## Features

- Route-only rendering path (no legacy elevation/HR/power chart modes)
- 3D extrusion + gradient route styling
- Route data endpoint for **client-side preview** (the web app renders the preview in-browser)
- Static PNG export (free tier adds a watermark; Pro is watermark-free)
- Pro MP4 video export (route draws itself frame-by-frame)
- Upload cache for fast re-renders
- Health endpoint for deployment monitoring

## Quick Start

```bash
docker-compose up --build
```

Service runs on `http://localhost:3000`.

## API

### 1) Upload

```bash
curl -X POST http://localhost:3000/api/upload \
  -F "file=@ride.gpx" \
  | jq
```

Example response:

```json
{
  "file_id": "a1b2c3d4-...",
  "file_type": "gpx",
  "metrics": {
    "distance_km": 82.4,
    "elevation_gain_m": 1240,
    "duration_seconds": 10800,
    "avg_speed_kmh": 27.4,
    "avg_heart_rate": 148,
    "max_heart_rate": 182,
    "avg_power": 220,
    "max_power": 410
  },
  "available_data": {
    "has_coordinates": true,
    "has_elevation": true,
    "has_heart_rate": true,
    "has_power": true
  }
}
```

### 2) Route data (for client-side preview)

The web app uses this endpoint to fetch normalized route points and then renders the live preview on a `<canvas>` in the browser.

```bash
curl "http://localhost:3000/api/route-data/a1b2c3d4-..."
```

### 3) Visualize (static PNG)

```bash
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "a1b2c3d4-...",
    "gradient": "rideviz",
    "color_by": "elevation",
    "stroke_width": 3,
    "padding": 40,
    "smoothing": 30,
    "glow": true
  }' \
  --output rideviz-route.png
```

`/api/visualize` returns `image/png`.

### 4) Export video (Pro MP4)

```bash
curl -X POST http://localhost:3000/api/export/video \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $LICENSE_TOKEN" \
  -d '{
    "file_id": "a1b2c3d4-...",
    "gradient": "rideviz",
    "color_by": "elevation",
    "stroke_width": 3,
    "padding": 40,
    "smoothing": 30,
    "glow": true,
    "background": "black",
    "duration_seconds": 6,
    "fps": 30
  }' \
  --output rideviz-route.mp4
```

`/api/export/video` returns `video/mp4` and requires a Pro license.

## Supported Options (PNG + MP4)

- `gradient`: `fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black`
- `color_by`: `elevation`, `speed`, `heartrate`, `power` (optional)
- `stroke_width`, `padding`, `smoothing`, `glow`

Server behavior is fixed to current product defaults:
- route visualization only
- 3D elevation extrusion
- preview is client-side (server provides route data + exports)

## Environment Variables

```bash
PORT=3000
MAX_FILE_SIZE_MB=25
CACHE_TTL_SECONDS=3600
VIDEO_EXPORT_MAX_CONCURRENCY=2
VIDEO_EXPORT_QUEUE_TIMEOUT_SECONDS=2
VIDEO_EXPORT_TIMEOUT_SECONDS=120
VIDEO_EXPORT_RATE_LIMIT_WINDOW_SECONDS=60
VIDEO_EXPORT_RATE_LIMIT_MAX_REQUESTS=4
RUST_LOG=info
```

## Development

```bash
cargo run
cargo test
cargo build --release
```
