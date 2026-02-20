#!/bin/bash

# Usage: ./test-render.sh <RENDER_URL> <GPX_FILE>
RENDER_URL="${1:-http://localhost:3000}"
GPX_FILE="${2:-test.gpx}"

echo "Testing RideViz-RS"
echo "URL: $RENDER_URL"
echo ""

echo "1) Health check"
if ! curl -s "$RENDER_URL/health" | jq '.'; then
  echo "Health check failed"
  exit 1
fi
echo ""

if [ ! -f "$GPX_FILE" ]; then
  echo "File not found: $GPX_FILE"
  exit 1
fi

echo "2) Uploading: $GPX_FILE"
UPLOAD_RESPONSE=$(curl -s -X POST "$RENDER_URL/api/upload" -F "file=@$GPX_FILE")
echo "$UPLOAD_RESPONSE" | jq '.'

FILE_ID=$(echo "$UPLOAD_RESPONSE" | jq -r '.file_id')
if [ "$FILE_ID" == "null" ] || [ -z "$FILE_ID" ]; then
  echo "Upload failed"
  exit 1
fi
echo "Upload successful: $FILE_ID"
echo ""

echo "3) Route 3D APNG (default profile)"
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"gradient\": \"rideviz\",
    \"color_by\": \"elevation\",
    \"stroke_width\": 3,
    \"padding\": 40,
    \"smoothing\": 30,
    \"glow\": true,
    \"animation_frames\": 100,
    \"animation_duration_ms\": 4600
  }" \
  --output route-3d.apng

if [ -f route-3d.apng ] && [ -s route-3d.apng ]; then
  SIZE=$(ls -lh route-3d.apng | awk '{print $5}')
  echo "route-3d.apng created ($SIZE)"
else
  echo "route-3d.apng failed"
  exit 1
fi

echo ""
echo "4) Route 3D APNG (no glow)"
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"gradient\": \"fire\",
    \"glow\": false,
    \"smoothing\": 50
  }" \
  --output route-3d-no-glow.apng

if [ -f route-3d-no-glow.apng ] && [ -s route-3d-no-glow.apng ]; then
  SIZE=$(ls -lh route-3d-no-glow.apng | awk '{print $5}')
  echo "route-3d-no-glow.apng created ($SIZE)"
else
  echo "route-3d-no-glow.apng failed"
  exit 1
fi

echo ""
echo "Done."
