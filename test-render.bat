@echo off
REM Test script for Render deployment (Windows)
REM Usage: test-render.bat <RENDER_URL> <GPX_FILE>

set RENDER_URL=%1
set GPX_FILE=%2

if "%RENDER_URL%"=="" set RENDER_URL=http://localhost:3000
if "%GPX_FILE%"=="" set GPX_FILE=test.gpx

echo Testing RideViz-RS on Render
echo ================================
echo URL: %RENDER_URL%
echo.

echo 1. Health Check...
curl -s "%RENDER_URL%/health"
echo.

echo 2. Uploading: %GPX_FILE%
curl -s -X POST "%RENDER_URL%/api/upload" -F "file=@%GPX_FILE%" > upload.json
type upload.json
echo.

REM Extract file_id from JSON (requires jq or manual copy)
echo Copy the file_id from above and run:
echo.
echo curl -X POST "%RENDER_URL%/api/visualize" -H "Content-Type: application/json" -d "{\"file_id\":\"FILE_ID_HERE\",\"type\":\"route\",\"format\":\"story\",\"gradient\":\"fire\"}" --output route.png
