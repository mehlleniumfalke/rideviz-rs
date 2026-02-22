import posthog from 'posthog-js';

type EventProperties = Record<string, string | number | boolean | null | undefined>;

type PostHogShim = {
  capture?: (event: string, properties?: Record<string, unknown>) => void;
  register?: (properties: Record<string, unknown>) => void;
  identify?: (distinctId: string) => void;
  get_distinct_id?: () => string;
};

const TRACKING_QUERY_KEYS = [
  'utm_source',
  'utm_medium',
  'utm_campaign',
  'utm_content',
  'utm_term',
  'gclid',
  'fbclid',
  'msclkid',
  'rv_src',
  'rv_cta',
  'rv_lid',
] as const;

type TrackingQueryKey = (typeof TRACKING_QUERY_KEYS)[number];
type TrackingContext = Partial<Record<TrackingQueryKey, string>>;

const STORAGE_FIRST_TOUCH = 'rideviz_posthog_first_touch';
const STORAGE_LAST_TOUCH = 'rideviz_posthog_last_touch';

function getPostHogClient(): PostHogShim {
  return posthog as unknown as PostHogShim;
}

function sanitizeProperties(
  properties: Record<string, string | number | boolean | null | undefined>,
): Record<string, string | number | boolean | null> {
  const sanitized: Record<string, string | number | boolean | null> = {};
  for (const [key, value] of Object.entries(properties)) {
    if (value !== undefined) {
      sanitized[key] = value;
    }
  }
  return sanitized;
}

function readStoredContext(storageKey: string): TrackingContext {
  if (typeof window === 'undefined') return {};
  const raw = localStorage.getItem(storageKey);
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const context: TrackingContext = {};
    for (const key of TRACKING_QUERY_KEYS) {
      const value = parsed[key];
      if (typeof value === 'string' && value.trim()) {
        context[key] = value.trim();
      }
    }
    return context;
  } catch {
    return {};
  }
}

function saveStoredContext(storageKey: string, context: TrackingContext): void {
  if (typeof window === 'undefined') return;
  localStorage.setItem(storageKey, JSON.stringify(context));
}

function readTrackingContextFromUrl(): TrackingContext {
  if (typeof window === 'undefined') return {};
  const search = new URLSearchParams(window.location.search);
  const context: TrackingContext = {};
  for (const key of TRACKING_QUERY_KEYS) {
    const value = search.get(key);
    if (value?.trim()) {
      context[key] = value.trim();
    }
  }
  return context;
}

function hasContext(context: TrackingContext): boolean {
  return Object.keys(context).length > 0;
}

function prefixedContext(
  prefix: 'first_touch_' | 'last_touch_',
  context: TrackingContext,
): Record<string, string> {
  const data: Record<string, string> = {};
  for (const [key, value] of Object.entries(context)) {
    if (typeof value === 'string' && value) {
      data[`${prefix}${key}`] = value;
    }
  }
  return data;
}

export function initAnalyticsContext(): void {
  if (typeof window === 'undefined') return;

  const current = readTrackingContextFromUrl();
  const firstTouch = readStoredContext(STORAGE_FIRST_TOUCH);
  const lastTouch = readStoredContext(STORAGE_LAST_TOUCH);

  if (hasContext(current)) {
    saveStoredContext(STORAGE_LAST_TOUCH, current);
    if (!hasContext(firstTouch)) {
      saveStoredContext(STORAGE_FIRST_TOUCH, current);
    }
  }

  const resolvedFirstTouch = hasContext(firstTouch) ? firstTouch : current;
  const resolvedLastTouch = hasContext(current) ? current : lastTouch;

  const ph = getPostHogClient();
  ph.register?.(
    sanitizeProperties({
      app_surface: 'rideviz_web',
      ...prefixedContext('first_touch_', resolvedFirstTouch),
      ...prefixedContext('last_touch_', resolvedLastTouch),
    }),
  );

  const handoffDistinctId = current.rv_lid;
  const currentDistinctId = ph.get_distinct_id?.();
  if (handoffDistinctId && currentDistinctId && handoffDistinctId !== currentDistinctId) {
    ph.identify?.(handoffDistinctId);
    captureEvent('rv_identity_handoff_applied', {
      handoff_source: current.rv_src ?? 'landing',
      handoff_cta: current.rv_cta ?? null,
    });
  }
}

export function captureEvent(event: string, properties: EventProperties = {}): void {
  const ph = getPostHogClient();
  ph.capture?.(
    event,
    sanitizeProperties({
      app_surface: 'rideviz_web',
      route_path: typeof window !== 'undefined' ? window.location.pathname : undefined,
      ...properties,
    }),
  );
}
