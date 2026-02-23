---
name: Visual Quality Improvements
overview: Improve the visual quality of rideviz route visualizations by adding drop shadows, gradient wall fades, a refined glow effect, enhanced endpoint markers, an optional 2D route shadow, and adaptive wall opacity. All changes touch the Rust SVG renderer and the React frontend controls.
todos:
  - id: drop-shadow
    content: Replace ground_path with a blurred drop shadow filter for softer depth grounding
    status: pending
  - id: wall-gradient-fade
    content: Change wall polygons from flat opacity to top-to-bottom gradient fade using per-bucket linearGradient defs
    status: pending
  - id: glow-improvement
    content: Refine glow filter to dual-layer (inner tight + outer faint), reduce stroke multiplier and opacity
    status: pending
  - id: background-composition
    content: Add subtle radial gradient background for white/black modes, wire background option through RenderOptions
    status: pending
  - id: endpoint-markers
    content: Add white stroke border and subtle drop shadow to start/end endpoint dots
    status: pending
  - id: route-shadow-option
    content: Add route_shadow bool option with blurred offset shadow path, wire through API + frontend checkbox
    status: pending
  - id: adaptive-wall-opacity
    content: Compute wall opacity from gradient average luminance instead of using fixed 0.24 constant
    status: pending
isProject: false
---

# Visual Quality Improvements

## Files overview

- **Backend renderer**: [src/pipeline/render.rs](src/pipeline/render.rs) — all SVG generation
- **Render options struct**: [src/types/viz.rs](src/types/viz.rs) — `RenderOptions`
- **API layer**: [src/routes/visualize.rs](src/routes/visualize.rs) — `VisualizeRequest`, `VideoExportRequest`, option wiring
- **Frontend types**: [rideviz-web/src/types/api.ts](rideviz-web/src/types/api.ts) — `VisualizeRequest` TS interface
- **Frontend controls**: [rideviz-web/src/pages/ToolPage/AdvancedPanel.tsx](rideviz-web/src/pages/ToolPage/AdvancedPanel.tsx) — checkbox/slider UI

---

## 1. Drop shadow / ambient occlusion for softer depth

Replace the flat white `ground_path` (opacity 0.14) with a blurred shadow path below the route that gives a sense of the route floating above the ground.

**In `render.rs**`:

- Add an SVG `<filter id="dropShadow">` with `feGaussianBlur` (stdDeviation ~~4-5) + `feOffset` (dx=0, dy=2-3) + `feFlood` with a dark color at low opacity (~~0.15)
- Apply this filter to the existing `ground_path`, changing its stroke color from white to a dark gray/black at ~0.20 opacity, and bumping its stroke width to ~1.5x the main stroke
- This replaces the barely-visible white ground trace with an actual shadow that grounds the route

No new options needed — this is a visual improvement to the existing ground path layer.

## 2. Gradient fade on wall polygons (top-to-bottom)

Currently walls use a flat `fill-opacity="0.24"`. Instead, use a per-wall vertical `linearGradient` that fades from opaque at the top (route line) to transparent at the ground.

**In `render.rs**`:

- In `build_wall_polygons` / `build_wall_polygons_precomputed`: for each wall polygon, instead of `fill-opacity="0.24"`, define a small linear gradient going from `(current.top.y)` to `(current.ground.y)` with:
  - Top stop: the wall color at opacity ~0.35
  - Bottom stop: the wall color at opacity ~0.04
- To avoid defining hundreds of `<linearGradient>` elements (one per polygon), bucket the wall heights into ~8-12 gradient defs and reuse them. Alternatively, use a single `linearGradient` in `userSpaceOnUse` coordinates with `gradientTransform` per polygon.
- A simpler approach: define one `<linearGradient id="wallFade" ...>` in `<defs>` that goes from `stop-opacity="1"` at 0% to `stop-opacity="0.1"` at 100%, oriented vertically. Then each polygon keeps its `fill="color"` and uses `fill-opacity` varying only slightly, while the gradient handles the fade. The cleanest SVG-compatible approach is to use per-polygon `<linearGradient>` with `gradientUnits="userSpaceOnUse"` and y1/y2 set to top/ground y coords. To keep it performant, batch polygons that share similar y-ranges into shared gradients.

**Recommended approach**: Use a single global vertical gradient definition per color bucket. Since we already bucket colors into `COLOR_BUCKETS=48`, create up to 48 `<linearGradient>` defs (one per used bucket) with `gradientUnits="objectBoundingBox"` going top-to-bottom (y1=0, y2=1, stop-opacity 0.35 -> 0.04). Each polygon references its bucket's gradient. This is clean and avoids per-polygon defs.

## 3. Improve the glow effect

Current glow: `stdDeviation="6"`, stroke 2.4x, opacity 0.6, double-merged blur. This is too thick and blobby.

**In `render.rs` `glow_filter_def()**`:

- Change to a dual-layer glow approach:
  - Inner glow: `stdDeviation="3"`, tighter
  - Outer glow: `stdDeviation="8"`, much fainter
- Updated filter:

```xml
<filter id="glow" x="-30%" y="-30%" width="160%" height="160%">
  <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="innerBlur"/>
  <feGaussianBlur in="SourceGraphic" stdDeviation="8" result="outerBlur"/>
  <feMerge>
    <feMergeNode in="outerBlur"/>
    <feMergeNode in="innerBlur"/>
    <feMergeNode in="SourceGraphic"/>
  </feMerge>
</filter>
```

- Reduce the glow path stroke multiplier from `2.4` to `1.8`
- Reduce the glow group opacity from `0.6` to `0.45`

## 4. Background / composition improvements

Add a subtle radial gradient background and vignette when background is `white` or `black`.

**In `render.rs**` in both `render_route_3d` and `render_svg_frame_precomputed`:

- Before the walls layer, add a `<rect>` with a `<radialGradient>` fill:
  - For white bg: center is pure white, edges are `#F0F0F0` (very subtle)
  - For black bg: center is `#1A1A1A`, edges are `#000000`
  - For transparent: skip
- This needs access to the background option. Add `background: Option<String>` to `RenderOptions`.

**In `viz.rs**`: Add `pub background: Option<String>` to `RenderOptions`, default `None`.

**In `visualize.rs**`: Wire the background string through to `RenderOptions` before rendering.

## 5. Endpoint markers improvement

Current dots are flat solid circles. Improve with a white border ring and a subtle drop shadow.

**In `render.rs` `render_endpoint_dots()**`:

- Add a shadow circle offset slightly (0, 1px) with blur, using a dark color at low opacity
- Add a white stroke border (`stroke="#FFFFFF" stroke-width="1.5"`) to each dot
- Slightly increase radius multiplier from `2.2` to `2.5`
- Add a `<filter id="dotShadow">` for the endpoint shadow

Updated output per dot:

```xml
<circle cx="..." cy="..." r="..." fill="black" opacity="0.15" filter="url(#dotShadow)"/>
<circle cx="..." cy="..." r="..." fill="COLOR" stroke="#FFFFFF" stroke-width="1.5" opacity="0.95"/>
```

## 6. Optional 2D drop shadow on the route

Add an option `route_shadow: bool` that renders a blurred, offset copy of the top path below it as a shadow, giving a "floating above paper" look.

**In `viz.rs**`: Add `pub route_shadow: bool` to `RenderOptions`, default `false`.

**In `render.rs**`: When `route_shadow` is true, render a shadow path right before the `outline_path`:

- Same path data as `top_path` but offset by (3, 3) pixels via `transform="translate(3,3)"`
- Dark color (`#000000`) at opacity 0.12
- Slightly wider stroke (1.3x)
- Apply a Gaussian blur filter (`stdDeviation="3"`)
- Add `<filter id="routeShadow">` def

**In `visualize.rs**`: Add `route_shadow` to `VisualizeRequest` / `VideoExportRequest` (default false), wire to options.

**In frontend**:

- `api.ts`: Add `route_shadow?: boolean` to `VisualizeRequest`
- `AdvancedPanel.tsx`: Add checkbox "Route shadow"

## 7. Adaptive wall polygon opacity

Make wall opacity scale based on gradient brightness so dark gradients (black, ocean) get more visible walls and bright gradients (white) get more subtle walls.

**In `render.rs**`:

- Add a helper `fn gradient_avg_luminance(gradient: &Gradient) -> f64` that computes the average perceived luminance of the gradient colors using `0.299*R + 0.587*G + 0.114*B`
- Replace the constant `WALL_FILL_OPACITY = 0.24` usage with a computed value:
  - High luminance (white): opacity ~0.12
  - Low luminance (black/dark): opacity ~0.35
  - Formula: `0.12 + (1.0 - luminance) * 0.23`
- This value is computed once per render call and passed to the wall builders

---

## SVG layer order (updated)

```
1. [optional] Background radial gradient rect
2. Wall polygons (with gradient fade)
3. Ground shadow path (drop shadow filter)
4. [optional] Route 2D shadow (if route_shadow=true)
5. Outline path (white border)
6. Glow path (improved dual-layer)
7. Top route path
8. Endpoint dots (with border + shadow)
9. Stats overlay
```

