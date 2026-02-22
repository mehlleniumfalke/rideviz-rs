import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import posthog from 'posthog-js'
import { initAnalyticsContext } from './analytics/posthog'
import './styles/index.css'
import App from './App.tsx'

const posthogApiKey = import.meta.env.VITE_POSTHOG_API_KEY

if (posthogApiKey) {
  posthog.init(posthogApiKey, {
    api_host: 'https://eu.i.posthog.com',
    capture_pageview: false, // manual pageview tracking below
  })
  initAnalyticsContext()
} else {
  console.error('[RideViz] Missing VITE_POSTHOG_API_KEY at build time.')
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
