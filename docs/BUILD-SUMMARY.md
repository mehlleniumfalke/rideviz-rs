# Build Summary

## Current Product Surface

RideViz-RS is now reduced to the essential backend for:

- GPX/FIT upload and processing
- route data preparation
- 3D route SVG rendering
- APNG animation output

## API Surface

- `GET /health`
- `POST /api/upload`
- `POST /api/visualize` (route-only)

`/api/visualize` accepts:

- `file_id`
- `gradient`
- `color_by?`
- `stroke_width?`
- `padding?`
- `smoothing?`
- `glow?`
- `animation_frames?`
- `animation_duration_ms?`

It returns `image/apng`.

## Notes

- Legacy non-route visualization modes are removed.
- Legacy request fields (`type`, `format`, `width/height`, `background`, `show_*`, etc.) are not part of the contract.
- Upload response no longer includes `available_visualizations`; use `available_data`.
