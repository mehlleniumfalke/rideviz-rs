@echo off
REM Usage: test-render.bat <RENDER_URL> <GPX_FILE>

set RENDER_URL=%1
set GPX_FILE=%2

if "%RENDER_URL%"=="" set RENDER_URL=http://localhost:3000
if "%GPX_FILE%"=="" set GPX_FILE=test.gpx

echo Testing RideViz-RS
echo URL: %RENDER_URL%
echo.

echo 1. Health check
curl -s "%RENDER_URL%/health"
echo.
echo.

echo 2. Upload
curl -s -X POST "%RENDER_URL%/api/upload" -F "file=@%GPX_FILE%" > upload.json
type upload.json
echo.
echo.

echo 3. Replace FILE_ID_HERE and run:
echo curl -X POST "%RENDER_URL%/api/visualize" -H "Content-Type: application/json" -d "{\"file_id\":\"FILE_ID_HERE\",\"gradient\":\"rideviz\",\"color_by\":\"elevation\",\"stroke_width\":3,\"padding\":40,\"smoothing\":30,\"glow\":true}" --output rideviz-route.png
