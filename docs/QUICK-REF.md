# RideViz-RS Quick Reference

## Quick Start

```bash
docker-compose up --build
```

Service: `http://localhost:3000`

## Endpoints

### Health

```bash
curl http://localhost:3000/health
```

### Upload

```bash
curl -X POST http://localhost:3000/api/upload \
  -F "file=@ride.gpx" \
  | jq
```

Returns `file_id`, `file_type`, `metrics`, `available_data`.

### Visualize (route 3D APNG)

```bash
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "uuid-from-upload",
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

## Visualize Options

| Parameter | Values | Default |
|-----------|--------|---------|
| `file_id` | string | required |
| `gradient` | `fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black` | `fire` |
| `color_by` | `elevation`, `speed`, `heartrate`, `power` | unset |
| `stroke_width` | number | `3.0` |
| `padding` | number | `40` |
| `smoothing` | `0-100` | `30` |
| `glow` | boolean | `true` |
| `animation_frames` | number | `100` |
| `animation_duration_ms` | number | `4600` |

## Fixed Backend Behavior

- route visualization only
- 3D animation only
- output is APNG (`image/apng`)
- render size fixed to `1920x1080`
- transparent background
