# Manual Testing Guide for Render Deployment

Replace `YOUR_RENDER_URL` with your actual Render URL (e.g., `https://rideviz-rs.onrender.com`)

## Step 1: Health Check

```bash
curl https://YOUR_RENDER_URL/health
```

Expected response:
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

If you get a 404 or connection error, your service might still be starting up. Wait 1-2 minutes and try again.

---

## Step 2: Upload GPX File

```bash
curl -X POST https://YOUR_RENDER_URL/api/upload \
  -F "file=@your-ride.gpx" \
  | jq
```

**OR on Windows PowerShell:**
```powershell
curl.exe -X POST https://YOUR_RENDER_URL/api/upload `
  -F "file=@your-ride.gpx"
```

Expected response:
```json
{
  "file_id": "a1b2c3d4-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "file_type": "gpx",
  "metrics": {
    "distance_km": 25.4,
    "elevation_gain_m": 340,
    "duration_seconds": 3600,
    "avg_speed_kmh": 25.4,
    "avg_heart_rate": 145,
    "max_heart_rate": 178,
    "avg_power": 220,
    "max_power": 450
  },
  "available_visualizations": ["route", "elevation", "heartrate", "power"]
}
```

**Copy the `file_id`** from the response for the next step.

---

## Step 3: Generate Visualizations

Replace `FILE_ID` with the ID from Step 2.

### Route Visualization
```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "fire"
  }' \
  --output route.png
```

**Note:** By default, routes include a glow effect and endpoint markers. These are enabled with `glow: true` and `show_endpoints: true`.

### Elevation Profile
```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "elevation",
    "format": "post",
    "gradient": "ocean"
  }' \
  --output elevation.png
```

### Heart Rate
```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "heartrate",
    "format": "wide",
    "gradient": "sunset"
  }' \
  --output heartrate.png
```

### Power
```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "power",
    "format": "story",
    "gradient": "violet"
  }' \
  --output power.png
```

---

## Step 4: Check Generated Files

```bash
ls -lh *.png
```

Open the PNG files to verify they look correct.

---

## Try Different Options

### Different Gradients
Available: `fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black`

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "rideviz"
  }' \
  --output route-rideviz.png
```

### Different Formats
Available: `story` (1080×1920), `post` (1080×1080), `wide` (1920×1080)

```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "wide",
    "gradient": "fire"
  }' \
  --output route-wide.png
```

### Custom Size
```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "custom",
    "width": 2000,
    "height": 2000,
    "gradient": "fire"
  }' \
  --output route-custom.png
```

### With Background Color
```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "white",
    "background": "#111111"
  }' \
  --output route-dark.png
```

### With Glow Effect Disabled
```bash
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "fire",
    "glow": false,
    "show_endpoints": false
  }' \
  --output route-clean.png
```

### With Custom Smoothing
```bash
# Low smoothing (raw GPS data)
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "fire",
    "smoothing": 0
  }' \
  --output route-raw.png

# Default smoothing (balanced)
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "fire",
    "smoothing": 30
  }' \
  --output route-balanced.png

# High smoothing (very smooth, stylized)
curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "fire",
    "smoothing": 100
  }' \
  --output route-smooth.png
```

---

## Available Parameters

### All Visualizations

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `file_id` | string | *required* | File ID from upload response |
| `type` | string | *required* | `route`, `elevation`, `heartrate`, or `power` |
| `format` | string | `story` | `story` (1080×1920), `post` (1080×1080), `wide` (1920×1080), or `custom` |
| `gradient` | string | `fire` | `fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black` |
| `background` | string | `transparent` | Hex color (e.g., `#111111`) or `transparent` |
| `stroke_width` | number | `3.0` | Line thickness in pixels |
| `padding` | number | `40` | Padding around visualization in pixels |

### Route-Specific Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `smoothing` | number | `30` | Smoothing level (0-100). 0=raw GPS, 30=balanced, 100=highly stylized |
| `glow` | boolean | `true` | Enable glow effect around the route line |
| `show_endpoints` | boolean | `true` | Show colored dots at start and end points |

### Time-Series Specific (elevation, heartrate, power)

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `smoothing` | number | `30` | Moving average window size for smoothing |
| `glow` | boolean | `true` | Enable glow effect around the line |

---

## Common Issues

### ❌ "Activity not found"
The file_id expired (1 hour cache). Upload again and use the new file_id.

### ❌ "No heart rate data available"
Your GPX file doesn't contain heart rate data. Try `route` or `elevation` instead.

### ❌ Connection timeout on first request
Render's free tier puts services to sleep after inactivity. First request wakes it up (~30s). Try again.

### ❌ 502 Bad Gateway
Service is still starting. Wait 1-2 minutes after deployment.

---

## Performance Check

Time a request:
```bash
time curl -X POST https://YOUR_RENDER_URL/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "FILE_ID",
    "type": "route",
    "format": "story"
  }' \
  --output test.png
```

Should complete in < 500ms after cache warmup.
