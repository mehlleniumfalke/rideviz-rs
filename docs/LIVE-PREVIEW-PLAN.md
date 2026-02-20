# Live Preview Plan (Route-Only)

## Goal

Provide a low-latency preview loop for the current route-3D animation product surface.

## Current Backend Contract

- `POST /api/upload`
- `POST /api/visualize` (route-only APNG)

`/api/visualize` request fields:
- `file_id`
- `gradient`
- `color_by?`
- `stroke_width?`
- `padding?`
- `smoothing?`
- `glow?`
- `animation_frames?`
- `animation_duration_ms?`

## Preview Strategy

1. Upload once and keep `file_id`.
2. Debounce UI changes (`~250-350ms`).
3. For preview, request reduced animation profile (fewer frames, shorter duration).
4. For download, request fuller profile.

## Optional Future Optimization

If APNG generation becomes a bottleneck, add `POST /api/preview` that returns SVG for rapid interactive updates while keeping `/api/visualize` as final APNG export.
