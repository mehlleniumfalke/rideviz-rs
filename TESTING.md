# Testing Guide

## Docker Build Test

```bash
# Build the image
docker-compose build

# This should compile all Rust code and create the binary
```

## Running the Service

```bash
# Start the service
docker-compose up

# Or run in background
docker-compose up -d

# Check logs
docker-compose logs -f
```

## API Testing

### 1. Health Check

```bash
curl http://localhost:3000/health
```

Expected response:
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

### 2. Upload Test File

You'll need a sample GPX or FIT file. Create a simple GPX:

```bash
cat > test.gpx << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="Test">
  <trk>
    <trkseg>
      <trkpt lat="37.7749" lon="-122.4194">
        <ele>10</ele>
        <time>2024-01-01T12:00:00Z</time>
        <extensions>
          <gpxtpx:hr>140</gpxtpx:hr>
          <gpxtpx:power>200</gpxtpx:power>
        </extensions>
      </trkpt>
      <trkpt lat="37.7750" lon="-122.4190">
        <ele>15</ele>
        <time>2024-01-01T12:00:30Z</time>
        <extensions>
          <gpxtpx:hr>145</gpxtpx:hr>
          <gpxtpx:power>210</gpxtpx:power>
        </extensions>
      </trkpt>
      <trkpt lat="37.7755" lon="-122.4185">
        <ele>20</ele>
        <time>2024-01-01T12:01:00Z</time>
        <extensions>
          <gpxtpx:hr>150</gpxtpx:hr>
          <gpxtpx:power>220</gpxtpx:power>
        </extensions>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
EOF
```

Upload it:

```bash
curl -X POST http://localhost:3000/api/upload \
  -F "file=@test.gpx" \
  | jq
```

Save the `file_id` from the response.

### 3. Generate Visualizations

```bash
# Route visualization
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "YOUR_FILE_ID",
    "type": "route",
    "format": "story",
    "gradient": "fire"
  }' \
  --output route.png

# Elevation visualization
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "YOUR_FILE_ID",
    "type": "elevation",
    "format": "post",
    "gradient": "ocean"
  }' \
  --output elevation.png

# Heart rate visualization
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "YOUR_FILE_ID",
    "type": "heartrate",
    "format": "wide",
    "gradient": "sunset"
  }' \
  --output heartrate.png

# Power visualization
curl -X POST http://localhost:3000/api/visualize \
  -H "Content-Type: application/json" \
  -d '{
    "file_id": "YOUR_FILE_ID",
    "type": "power",
    "format": "story",
    "gradient": "violet"
  }' \
  --output power.png
```

## Troubleshooting

### Container won't build

Check Docker logs:
```bash
docker-compose logs
```

### Port 3000 already in use

Change the port in `docker-compose.yml`:
```yaml
ports:
  - "3001:3000"  # Use 3001 on host
```

### Out of memory during build

Increase Docker memory limit in Docker Desktop settings.

## Performance Testing

```bash
# Time a full upload + visualize cycle
time (
  FILE_ID=$(curl -s -X POST http://localhost:3000/api/upload \
    -F "file=@test.gpx" | jq -r '.file_id')
  
  curl -s -X POST http://localhost:3000/api/visualize \
    -H "Content-Type: application/json" \
    -d "{\"file_id\":\"$FILE_ID\",\"type\":\"route\",\"format\":\"story\"}" \
    --output /dev/null
)
```

Expected: < 100ms for small files
