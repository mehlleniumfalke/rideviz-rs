import type {
  CheckoutResponse,
  LicenseResponse,
  LicenseVerifyResponse,
  RouteDataResponse,
  StravaActivitySummary,
  StravaAuthResponse,
  StravaCallbackResponse,
  UploadResponse,
  VideoExportRequest,
  VisualizeRequest,
} from '../types/api';

const apiBase = (import.meta.env.VITE_RIDEVIZ_API_BASE_URL as string | undefined)?.replace(/\/$/, '') ?? '';

function buildUrl(path: string): string {
  return apiBase ? `${apiBase}${path}` : path;
}

async function readError(response: Response): Promise<string> {
  try {
    const parsed = (await response.json()) as { error?: string; message?: string };
    return parsed.error ?? parsed.message ?? `Request failed (${response.status})`;
  } catch {
    return `Request failed (${response.status})`;
  }
}

export async function uploadFile(file: File): Promise<UploadResponse> {
  const formData = new FormData();
  formData.append('file', file);

  const response = await fetch(buildUrl('/api/upload'), {
    method: 'POST',
    body: formData,
  });

  if (!response.ok) {
    throw new Error(await readError(response));
  }

  return (await response.json()) as UploadResponse;
}

export async function getVisualization(
  payload: VisualizeRequest,
  signal?: AbortSignal,
  authToken?: string,
): Promise<Blob> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (authToken) {
    headers.Authorization = `Bearer ${authToken}`;
  }

  const response = await fetch(buildUrl('/api/visualize'), {
    method: 'POST',
    headers,
    body: JSON.stringify(payload),
    signal,
  });

  if (!response.ok) {
    throw new Error(await readError(response));
  }

  return response.blob();
}

export async function exportVideo(
  payload: VideoExportRequest,
  authToken: string,
): Promise<Blob> {
  const response = await fetch(buildUrl('/api/export/video'), {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${authToken}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(await readError(response));
  }

  return response.blob();
}

export async function getRouteData(
  fileId: string,
  options?: { colorBy?: string | null; smoothing?: number },
): Promise<RouteDataResponse> {
  const query = new URLSearchParams();
  if (options?.colorBy) {
    query.set('color_by', options.colorBy);
  }
  if (typeof options?.smoothing === 'number') {
    query.set('smoothing', String(options.smoothing));
  }

  const queryPart = query.toString() ? `?${query.toString()}` : '';
  const response = await fetch(buildUrl(`/api/route-data/${fileId}${queryPart}`));
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as RouteDataResponse;
}

export async function createCheckoutSession(email: string): Promise<CheckoutResponse> {
  const response = await fetch(buildUrl('/api/checkout'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email }),
  });

  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as CheckoutResponse;
}

export async function verifyLicense(authToken: string): Promise<LicenseVerifyResponse> {
  const response = await fetch(buildUrl('/api/license/verify'), {
    headers: {
      Authorization: `Bearer ${authToken}`,
    },
  });
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as LicenseVerifyResponse;
}

export async function handleStripeWebhookCompletion(payload: unknown): Promise<LicenseResponse> {
  const response = await fetch(buildUrl('/api/webhook/stripe'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as LicenseResponse;
}

export async function completeCheckoutSession(sessionId: string): Promise<LicenseResponse> {
  const response = await fetch(buildUrl('/api/checkout/complete'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ session_id: sessionId }),
  });
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as LicenseResponse;
}

export async function getStravaAuthUrl(): Promise<StravaAuthResponse> {
  const response = await fetch(buildUrl('/api/strava/auth'));
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as StravaAuthResponse;
}

export async function completeStravaAuth(
  code: string,
  state: string,
): Promise<StravaCallbackResponse> {
  const response = await fetch(
    buildUrl(`/api/strava/callback?code=${encodeURIComponent(code)}&state=${encodeURIComponent(state)}`),
  );
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as StravaCallbackResponse;
}

export async function listStravaActivities(authToken: string): Promise<StravaActivitySummary[]> {
  const response = await fetch(buildUrl('/api/strava/activities'), {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as StravaActivitySummary[];
}

export async function importStravaActivity(
  authToken: string,
  activityId: number,
): Promise<UploadResponse> {
  const response = await fetch(buildUrl(`/api/strava/activity/${activityId}`), {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  if (!response.ok) {
    throw new Error(await readError(response));
  }
  return (await response.json()) as UploadResponse;
}
