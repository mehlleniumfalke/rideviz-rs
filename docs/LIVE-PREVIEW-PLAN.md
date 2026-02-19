# Live Preview — Implementation Plan

## Goal

Add a landing page with a live preview that updates the rendered visualization in real-time as the user adjusts parameters (gradient, background, glow, smoothing, etc.).

---

## Current State

- **Backend only** — Axum API with two endpoints: `POST /api/upload` (returns `file_id`) and `POST /api/visualize` (returns PNG).
- **No frontend**, no static file serving.
- Pipeline: Parse → Process → Prepare → Render (SVG string) → Rasterize (PNG).
- Rasterization (`usvg` + `resvg` + `tiny-skia`) is the slowest step.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Landing Page (static HTML/CSS/JS)                  │
│                                                     │
│  ┌──────────┐  ┌──────────────────────────────────┐ │
│  │  Upload   │  │  Controls                        │ │
│  │  (.gpx/.  │  │  gradient | bg | glow | stroke  │ │
│  │   fit)    │  │  smoothing | format | endpoints  │ │
│  └──────────┘  └──────────────────────────────────┘ │
│                                                     │
│  ┌──────────────────────────────────────────────────┐│
│  │  Live Preview (<img> / inline SVG)               ││
│  │  ← updates on every param change (debounced)     ││
│  └──────────────────────────────────────────────────┘│
│                                                     │
│  [ Download Full-Res PNG ]                          │
└─────────────────────────────────────────────────────┘
         │                          │
         │ POST /api/upload         │ POST /api/preview (SVG)
         │                          │ POST /api/visualize (PNG)
         ▼                          ▼
┌─────────────────────────────────────────────────────┐
│  Axum Backend                                       │
│  /api/upload     → parse + cache                    │
│  /api/preview    → prepare + render → SVG (NEW)     │
│  /api/visualize  → prepare + render + rasterize     │
│  /               → serve static frontend files      │
└─────────────────────────────────────────────────────┘
```

---

## Implementation Steps

### Phase 1: Backend Changes

#### 1.1 — Add SVG preview endpoint

New route: `POST /api/preview`

- Same request body as `/api/visualize`
- Skips the rasterize step — returns the raw SVG string directly
- Response: `Content-Type: image/svg+xml`
- The SVG already contains correct `width`/`height`/`viewBox` — browsers render it natively
- Background color is applied via a `<rect>` element prepended to the SVG (instead of via pixmap fill)

Why: SVG responses are ~10-50x faster than PNG. No rasterization, smaller payload. The browser handles rendering.

**Files to change:**
- `src/routes/visualize.rs` — add `preview` handler (or new file `src/routes/preview.rs`)
- `src/routes/mod.rs` — register the new route
- `src/main.rs` — merge the new router

#### 1.2 — Add static file serving

Serve the frontend from a `static/` directory using `tower-http::ServeDir`.

**Files to change:**
- `Cargo.toml` — add `"fs"` feature to `tower-http`
- `src/main.rs` — add fallback service: `Router::new().fallback_service(ServeDir::new("static"))`

---

### Phase 2: Frontend (single-page, vanilla JS)

All files in `static/` directory. No build step, no framework.

#### 2.1 — `static/index.html`

Landing page structure:
- Header with branding
- File upload area (drag & drop + file picker)
- Controls panel (appears after upload)
- Preview area (appears after upload)
- Download button

#### 2.2 — Controls

| Control | Type | Values |
|---------|------|--------|
| Visualization type | Buttons/tabs | `route`, `elevation`, `heartrate`, `power` (only show available ones from upload response) |
| Format | Select | `story` (1080×1920), `post` (1080×1080), `wide` (1920×1080) |
| Gradient | Color swatches | `fire`, `ocean`, `sunset`, `forest`, `violet`, `rideviz`, `white`, `black` |
| Background | Color picker + transparent toggle | `transparent`, `#RRGGBB` |
| Stroke width | Range slider | 1.0 – 10.0, default 3.0 |
| Smoothing | Range slider | 0 – 100, default 30 |
| Glow | Toggle | on/off, default on |
| Endpoints | Toggle | on/off, default on |

#### 2.3 — Preview logic (JS)

```
upload file
  → POST /api/upload
  → store file_id + available visualizations
  → show controls
  → trigger initial preview

on any control change (debounced 300ms):
  → POST /api/preview { file_id, type, gradient, ... }
  → display returned SVG inline or as <img src="data:image/svg+xml,...">

on "Download" click:
  → POST /api/visualize { file_id, type, gradient, ... }
  → trigger browser download of PNG blob
```

#### 2.4 — `static/style.css`

Dark theme (matches the visualization aesthetic), responsive layout, modern controls.

---

### Phase 3: Polish

- **Loading indicator** — skeleton/spinner while preview fetches
- **Error states** — file too large, unsupported format, expired cache
- **Responsive** — preview scales down on mobile, controls stack vertically
- **Keyboard shortcuts** — arrow keys on sliders, tab through controls
- **URL state** — persist params in URL hash so preview links are shareable (post-upload)

---

## File Structure (new files)

```
static/
├── index.html
├── style.css
└── app.js
src/routes/
├── preview.rs          (new)
```

## Modified Files

```
Cargo.toml              (add "fs" feature to tower-http)
src/main.rs             (add static serving + preview route)
src/routes/mod.rs       (register preview module)
src/routes/visualize.rs (optional: extract shared logic)
```

---

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| SVG preview vs PNG preview | SVG | 10-50x faster, no rasterization cost, browser renders natively |
| Vanilla JS vs framework | Vanilla | Single page, no build step, minimal complexity, fast to ship |
| Debounce interval | 300ms | Fast enough to feel live, avoids spamming the server |
| Background in SVG | Prepend `<rect>` | Can't use pixmap fill without rasterizing; a rect works for SVG preview |
| Static serving vs separate frontend | Same server | Single deployment, no CORS config needed, simpler Docker setup |

---

## Estimated Effort

| Task | Estimate |
|------|----------|
| 1.1 SVG preview endpoint | ~30 min |
| 1.2 Static file serving | ~10 min |
| 2.1-2.4 Frontend page | ~2-3 hours |
| 3 Polish | ~1-2 hours |
| **Total** | **~4-6 hours** |
