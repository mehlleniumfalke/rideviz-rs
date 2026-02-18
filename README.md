# RideViz-RS

High-performance Rust implementation of RideViz — converts GPS activity files (GPX, FIT) into beautiful transparent PNG visualizations.

## Features

- **Fast**: Parse and render in ~30-60ms (vs 1-3s in Node.js version)
- **No browser needed**: Uses native SVG → PNG rendering via `resvg`
- **Transparent PNGs**: Perfect for video overlays (Instagram Stories, TikTok, YouTube)
- **4 visualization types**: Route, Elevation, Heart Rate, Power
- **Smart caching**: Parse once, visualize unlimited times
- **Type-safe**: 5-stage typed pipeline with precise error handling

## Quick Start with Docker

```bash
# Build and run
docker-compose up --build

# The service will be available at http://localhost:3000
```

## API Usage

### 1. Upload a file

```bash
curl -X POST http://localhost:3000/api/upload \
  -F "file=@ride.gpx" \
  | jq
```

Response:
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
    "max_heart_rate": 182
  },
  "available_visualizations": ["route", "elevation", "heartrate", "power"]
}
```

### 2. Generate visualization

```bash
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "a1b2c3d4-...",
    "type": "route",
    "format": "story",
    "gradient": "fire"
  }' \
  --output route.png
```

## Visualization Types

- `route` — 2D map of your ride (requires GPS coordinates)
- `elevation` — Elevation profile over distance
- `heartrate` — Heart rate over time
- `power` — Power output over time

## Format Presets

- `story` — 1080×1920 (Instagram Stories, TikTok, Reels)
- `post` — 1080×1080 (Instagram feed)
- `wide` — 1920×1080 (YouTube thumbnail)
- `custom` — Specify `width` and `height` manually

## Gradients

`fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black`

## Environment Variables

```bash
PORT=3000                    # Server port
MAX_FILE_SIZE_MB=25         # Max upload size
CACHE_TTL_SECONDS=3600      # Cache expiration (1 hour)
RUST_LOG=info               # Log level
```

## Architecture

The pipeline flows through 5 typed stages:

```
Stage 1: Parse     → Raw bytes to TrackPoints
Stage 2: Process   → Compute metrics, downsample (cached here)
Stage 3: Prepare   → Project data for specific viz type
Stage 4: Render    → Generate SVG
Stage 5: Rasterize → Convert SVG to PNG
```

## Development

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run locally
cargo run

# Run tests
cargo test

# Build release
cargo build --release
```

## Performance

Typical request times on modest hardware (2-core, 4GB):

| Operation | Time |
|-----------|------|
| Parse GPX (5000 points) | ~3-5ms |
| Process (metrics + downsample) | ~0.5ms |
| Prepare (projection) | ~0.2ms |
| Render SVG | ~1-2ms |
| Rasterize PNG (1080×1920) | ~20-50ms |
| **Total** | **~30-60ms** |

## License

MIT
