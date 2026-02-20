# Testing Guide

## Build

```bash
docker-compose build
```

## Run

```bash
docker-compose up
```

## API Checks

### 1) Health

```bash
curl http://localhost:3000/health
```

Expected:

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### 2) Upload

```bash
curl -X POST http://localhost:3000/api/upload \
  -F "file=@test.gpx" \
  | jq
```

Copy `file_id`.

### 3) Route 3D animation

```bash
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "YOUR_FILE_ID",
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

Expected: `route-3d.apng` exists and is non-empty.

## Performance Sanity Check

```bash
time (
  FILE_ID=$(curl -s -X POST http://localhost:3000/api/upload \
    -F "file=@test.gpx" | jq -r '.file_id')

  curl -s -X POST http://localhost:3000/api/visualize \
    -H "Content-Type: application/json" \
    -d "{\"file_id\":\"$FILE_ID\"}" \
    --output /dev/null
)
```
