import type { UploadResponse, VisualizeRequest } from '../types/api';

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
): Promise<Blob> {
  const response = await fetch(buildUrl('/api/visualize'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(payload),
    signal,
  });

  if (!response.ok) {
    throw new Error(await readError(response));
  }

  return response.blob();
}
