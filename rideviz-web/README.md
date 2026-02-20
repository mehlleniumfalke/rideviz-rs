# RideViz Web

Frontend implementation for the build plan in `docs/RIDEVIZ_RS_FRONTEND.md`.

This app uses one Vite + React + TypeScript project for both routes:

- `/` landing page
- `/app` visualization tool

## Run

```bash
npm install
npm run dev
```

## Build

```bash
npm run build
```

## API base URL

By default, API requests go to same-origin paths:

- `POST /api/upload`
- `POST /api/visualize`

To call a separate backend domain (for example `https://api.rideviz.app`), set:

```bash
VITE_RIDEVIZ_API_BASE_URL=https://api.rideviz.app
```
