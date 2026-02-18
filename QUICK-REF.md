# RideViz-RS Quick Reference

## 🚀 Quick Start

```bash
cd rideviz-rs
docker-compose up --build
```

Service runs on: **http://localhost:3000**

## 📋 API Cheatsheet

### Health Check
```bash
curl http://localhost:3000/health
```

### Upload File
```bash
curl -X POST http://localhost:3000/api/upload \
  -F "file=@ride.gpx" \
  | jq
```

Returns:
```json
{
  "file_id": "uuid-here",
  "file_type": "gpx",
  "metrics": { ... },
  "available_visualizations": ["route", "elevation", "heartrate", "power"]
}
```

### Generate PNG
```bash
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "uuid-from-upload",
    "type": "route",
    "format": "story",
    "gradient": "fire"
  }' \
  --output viz.png
```

## 🎨 Options

| Parameter | Values | Default |
|-----------|--------|---------|
| `type` | `route`, `elevation`, `heartrate`, `power` | (required) |
| `format` | `story`, `post`, `wide`, `custom` | `story` |
| `gradient` | `fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black` | `fire` |
| `width` | number | (format preset) |
| `height` | number | (format preset) |
| `stroke_width` | number | `3.0` |
| `padding` | number | `40` |
| `smoothing` | number | `5` |
| `background` | `transparent` or `#RRGGBB` | `transparent` |

## 📐 Format Presets

- **story**: 1080×1920 (Instagram Stories, TikTok, Reels)
- **post**: 1080×1080 (Instagram feed)
- **wide**: 1920×1080 (YouTube thumbnail)
- **custom**: Specify width/height

## 🔧 Environment Variables

```bash
PORT=3000                  # Server port
MAX_FILE_SIZE_MB=25       # Upload limit
CACHE_TTL_SECONDS=3600    # 1 hour cache
RUST_LOG=info             # Log level
```

## 📁 Project Structure

```
rideviz-rs/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Config
│   ├── error.rs             # Errors
│   ├── state.rs             # Cache
│   ├── types/               # Data models
│   ├── pipeline/            # 5 stages
│   │   ├── parse/           # Stage 1: GPX/FIT → ParsedActivity
│   │   ├── process.rs       # Stage 2: → ProcessedActivity (cached)
│   │   ├── prepare.rs       # Stage 3: → VizData
│   │   ├── render.rs        # Stage 4: → SVG
│   │   └── rasterize.rs     # Stage 5: → PNG
│   └── routes/              # API endpoints
├── Cargo.toml               # Dependencies
├── Dockerfile               # Container build
└── docker-compose.yml       # Easy startup
```

## ⚡ Performance

Expected timings (5000 point file):
- Parse: ~3-5ms
- Process: ~0.5ms
- Prepare: ~0.2ms
- Render: ~1-2ms
- Rasterize: ~20-50ms
- **Total: ~30-60ms** (vs 1-3s in Node.js)

## 🧪 Testing

See `TESTING.md` for detailed test scenarios.

Quick test:
```bash
# Create test file
cat > test.gpx << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1">
  <trk><trkseg>
    <trkpt lat="37.7749" lon="-122.4194"><ele>10</ele></trkpt>
    <trkpt lat="37.7755" lon="-122.4185"><ele>20</ele></trkpt>
  </trkseg></trk>
</gpx>
EOF

# Upload and visualize
FILE_ID=$(curl -s -X POST http://localhost:3000/api/upload -F "file=@test.gpx" | jq -r '.file_id')
curl -X POST http://localhost:3000/api/visualize -H "Content-Type: application/json" \
  -d "{\"file_id\":\"$FILE_ID\",\"type\":\"route\",\"format\":\"story\"}" \
  --output test.png
```

## 📦 Dependencies

Core libraries:
- **axum** — HTTP server
- **tokio** — Async runtime
- **resvg** — SVG to PNG (no browser!)
- **quick-xml** — GPX parser
- **fitparser** — FIT parser
- **dashmap** — Concurrent cache

## 🎯 Key Features

✅ No browser (native rendering)  
✅ Transparent PNGs  
✅ Smart caching  
✅ Type-safe pipeline  
✅ 4 visualization types  
✅ 8 gradient presets  
✅ Sub-100ms responses  
✅ Docker ready  

## 📚 Documentation

- `README.md` — Full guide
- `TESTING.md` — Test scenarios
- `BUILD-SUMMARY.md` — Implementation details
- `QUICK-REF.md` — This file

## 🐛 Troubleshooting

**Port in use?**
```yaml
# docker-compose.yml
ports:
  - "3001:3000"
```

**Check logs:**
```bash
docker-compose logs -f
```

**Rebuild:**
```bash
docker-compose down
docker-compose up --build
```
