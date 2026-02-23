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

## 3) Static PNG export

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
    "glow": true
  }' \
  --output rideviz-route.png
```

## 4) Verify output

```bash
ls -lh rideviz-route.png
```

Open the PNG and verify:
- transparent background
- 3D elevated route effect

## Option checks

### Alternate gradient

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","gradient":"fire"}' \
  --output route-fire.png
```

### Disable glow

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","glow":false}' \
  --output route-no-glow.png
```

### Smoothing variants

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","smoothing":0}' \
  --output route-raw.png

curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{"file_id":"FILE_ID","smoothing":100}' \
  --output route-smooth.png
```
