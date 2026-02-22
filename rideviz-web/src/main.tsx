import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import posthog from 'posthog-js'
import { initAnalyticsContext } from './analytics/posthog'
import './styles/index.css'
import App from './App.tsx'

posthog.init(import.meta.env.VITE_POSTHOG_API_KEY, {
  api_host: 'https://eu.i.posthog.com',
  capture_pageview: false, // manual pageview tracking below
})
initAnalyticsContext()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
