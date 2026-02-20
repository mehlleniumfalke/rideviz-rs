# Manual Testing Guide for Render Deployment

Replace `YOUR_RENDER_URL` with your Render URL.

## 1) Health

```bash
curl https://YOUR_RENDER_URL/health
```

Expected:

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

## 2) Upload

```bash
curl -X POST https://YOUR_RENDER_URL/api/upload \
  -F "file=@your-ride.gpx" \
  | jq
```

Expected fields:
- `file_id`
- `file_type`
- `metrics`
- `available_data`

## 3) Route 3D APNG

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
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

## 4) Verify output

```bash
ls -lh route-3d.apng
```

Open the APNG and verify:
- transparent background
- 3D elevated route effect
- animation plays smoothly

## Option checks

### Alternate gradient

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","gradient":"fire"}' \
  --output route-fire.apng
```

### Disable glow

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","glow":false}' \
  --output route-no-glow.apng
```

### Smoothing variants

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","smoothing":0}' \
  --output route-raw.apng

curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","smoothing":100}' \
  --output route-smooth.apng
```
