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

echo "3) Static PNG (default)"
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"gradient\": \"rideviz\",
    \"color_by\": \"elevation\",
    \"stroke_width\": 3,
    \"padding\": 40,
    \"smoothing\": 30,
    \"glow\": true
  }" \
  --output rideviz-route.png

if [ -f rideviz-route.png ] && [ -s rideviz-route.png ]; then
  SIZE=$(ls -lh rideviz-route.png | awk '{print $5}')
  echo "rideviz-route.png created ($SIZE)"
else
  echo "rideviz-route.png failed"
  exit 1
fi

echo ""
echo "4) Static PNG (no glow)"
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"gradient\": \"fire\",
    \"glow\": false,
    \"smoothing\": 50
  }" \
  --output rideviz-route-no-glow.png

if [ -f rideviz-route-no-glow.png ] && [ -s rideviz-route-no-glow.png ]; then
  SIZE=$(ls -lh rideviz-route-no-glow.png | awk '{print $5}')
  echo "rideviz-route-no-glow.png created ($SIZE)"
else
  echo "rideviz-route-no-glow.png failed"
  exit 1
fi

echo ""
echo "Done."
