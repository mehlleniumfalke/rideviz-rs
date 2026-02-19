#!/bin/bash

# Test script for Render deployment
# Usage: ./test-render.sh <RENDER_URL> <GPX_FILE>
# Example: ./test-render.sh https://rideviz-rs.onrender.com ride.gpx

RENDER_URL="${1:-http://localhost:3000}"
GPX_FILE="${2:-test.gpx}"

echo "🧪 Testing RideViz-RS on Render"
echo "================================"
echo "URL: $RENDER_URL"
echo ""

# Test 1: Health check
echo "1️⃣  Health Check..."
curl -s "$RENDER_URL/health" | jq '.'
if [ $? -eq 0 ]; then
  echo "✅ Health check passed"
else
  echo "❌ Health check failed - is the service running?"
  exit 1
fi
echo ""

# Test 2: Upload GPX file
if [ ! -f "$GPX_FILE" ]; then
  echo "❌ File not found: $GPX_FILE"
  echo "Usage: $0 <RENDER_URL> <GPX_FILE>"
  exit 1
fi

echo "2️⃣  Uploading: $GPX_FILE"
UPLOAD_RESPONSE=$(curl -s -X POST "$RENDER_URL/api/upload" -F "file=@$GPX_FILE")
echo "$UPLOAD_RESPONSE" | jq '.'

FILE_ID=$(echo "$UPLOAD_RESPONSE" | jq -r '.file_id')
if [ "$FILE_ID" == "null" ] || [ -z "$FILE_ID" ]; then
  echo "❌ Upload failed"
  exit 1
fi
echo "✅ Upload successful! File ID: $FILE_ID"
echo ""

# Test 3: Generate visualizations
echo "3️⃣  Generating visualizations..."
echo ""

# Route (default with glow and endpoints)
echo "📍 Route (story format, fire gradient, glow enabled)..."
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"type\": \"route\",
    \"format\": \"story\",
    \"gradient\": \"fire\"
  }" \
  --output route.png

if [ -f route.png ] && [ -s route.png ]; then
  SIZE=$(ls -lh route.png | awk '{print $5}')
  echo "✅ route.png created ($SIZE)"
else
  echo "❌ route.png failed"
fi

# Route without glow
echo "📍 Route (post format, ocean gradient, no glow)..."
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"type\": \"route\",
    \"format\": \"post\",
    \"gradient\": \"ocean\",
    \"glow\": false,
    \"show_endpoints\": false
  }" \
  --output route-clean.png

if [ -f route-clean.png ] && [ -s route-clean.png ]; then
  SIZE=$(ls -lh route-clean.png | awk '{print $5}')
  echo "✅ route-clean.png created ($SIZE)"
else
  echo "❌ route-clean.png failed"
fi

# Elevation
echo "📈 Elevation (post format, ocean gradient)..."
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"type\": \"elevation\",
    \"format\": \"post\",
    \"gradient\": \"ocean\"
  }" \
  --output elevation.png

if [ -f elevation.png ] && [ -s elevation.png ]; then
  SIZE=$(ls -lh elevation.png | awk '{print $5}')
  echo "✅ elevation.png created ($SIZE)"
else
  echo "❌ elevation.png failed"
fi

# Heart Rate (if available)
echo "❤️  Heart Rate (wide format, sunset gradient)..."
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"type\": \"heartrate\",
    \"format\": \"wide\",
    \"gradient\": \"sunset\"
  }" \
  --output heartrate.png 2>/dev/null

if [ -f heartrate.png ] && [ -s heartrate.png ]; then
  SIZE=$(ls -lh heartrate.png | awk '{print $5}')
  echo "✅ heartrate.png created ($SIZE)"
else
  echo "⚠️  heartrate.png skipped (no HR data in file)"
  rm -f heartrate.png 2>/dev/null
fi

# Power (if available)
echo "⚡ Power (story format, violet gradient)..."
curl -s -X POST "$RENDER_URL/api/visualize" \
  -H "Content-Type: application/json" \
  -d "{
    \"file_id\": \"$FILE_ID\",
    \"type\": \"power\",
    \"format\": \"story\",
    \"gradient\": \"violet\"
  }" \
  --output power.png 2>/dev/null

if [ -f power.png ] && [ -s power.png ]; then
  SIZE=$(ls -lh power.png | awk '{print $5}')
  echo "✅ power.png created ($SIZE)"
else
  echo "⚠️  power.png skipped (no power data in file)"
  rm -f power.png 2>/dev/null
fi

echo ""
echo "================================"
echo "🎉 Testing complete!"
echo ""
echo "Generated files:"
ls -lh *.png 2>/dev/null || echo "No PNG files created"
echo ""
echo "💡 Tip: Try different smoothing values (0-100):"
echo "   smoothing: 0   = raw GPS points, no smoothing"
echo "   smoothing: 30  = default, balanced (new default)"
echo "   smoothing: 100 = heavily stylized, very rounded"
