# RideViz-RS

High-performance Rust backend for **3D animated route overlays** from GPX/FIT activities.

## Features

- Route-only rendering path (no legacy elevation/HR/power chart modes)
- 3D extrusion + gradient route styling
- APNG animation output for transparent overlays
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

### 2) Visualize (route-only APNG)

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
    "glow": true,
    "animation_frames": 100,
    "animation_duration_ms": 4600
  }' \
  --output route-3d.apng
```

`/api/visualize` returns `image/apng`.

## Supported Options

- `gradient`: `fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black`
- `color_by`: `elevation`, `speed`, `heartrate`, `power` (optional)
- `stroke_width`, `padding`, `smoothing`, `glow`
- `animation_frames`, `animation_duration_ms`

Server behavior is fixed to current product defaults:
- wide canvas (`1920x1080`)
- transparent background
- 3D route animation output

## Environment Variables

```bash
PORT=3000
MAX_FILE_SIZE_MB=25
CACHE_TTL_SECONDS=3600
RUST_LOG=info
```

## Development

```bash
cargo run
cargo test
cargo build --release
```
