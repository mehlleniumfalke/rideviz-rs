import { useEffect, useMemo, useState } from 'react';
import {
  completeStravaAuth,
  getStravaAuthUrl,
  importStravaActivity,
  listStravaActivities,
} from '../../api/client';
import type { StravaActivitySummary, UploadResponse } from '../../types/api';

interface StravaConnectProps {
  onImported: (response: UploadResponse) => void;
  enabled: boolean;
}

const STORAGE_KEY_STRAVA_TOKEN = 'rideviz_strava_token';

export default function StravaConnect({ onImported, enabled }: StravaConnectProps) {
  const [loading, setLoading] = useState(false);
  const [token, setToken] = useState<string | null>(localStorage.getItem(STORAGE_KEY_STRAVA_TOKEN));
  const [activities, setActivities] = useState<StravaActivitySummary[]>([]);
  const [error, setError] = useState<string | null>(null);

  const hasActivities = useMemo(() => activities.length > 0, [activities]);

  useEffect(() => {
    if (!enabled) return;
    const params = new URLSearchParams(window.location.search);
    const code = params.get('code');
    const state = params.get('state');
    if (!code || !state) return;

    let cancelled = false;
    setLoading(true);
    setError(null);
    const finishOAuth = async () => {
      try {
        const callback = await completeStravaAuth(code, state);
        if (cancelled) return;
        localStorage.setItem(STORAGE_KEY_STRAVA_TOKEN, callback.access_token);
        setToken(callback.access_token);
        const fetched = await listStravaActivities(callback.access_token);
        if (!cancelled) {
          setActivities(fetched);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to connect Strava');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
          params.delete('code');
          params.delete('state');
          params.delete('scope');
          const query = params.toString();
          const nextUrl = `${window.location.pathname}${query ? `?${query}` : ''}`;
          window.history.replaceState({}, '', nextUrl);
        }
      }
    };

    void finishOAuth();
    return () => {
      cancelled = true;
    };
  }, [enabled]);

  const handleConnect = async () => {
    if (!enabled) return;
    setLoading(true);
    setError(null);
    try {
      const auth = await getStravaAuthUrl();
      // Redirect in the same tab so OAuth callback returns to /app and is handled automatically.
      window.location.assign(auth.auth_url);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to connect Strava');
      setLoading(false);
    }
  };

  const handleRefreshActivities = async () => {
    if (!enabled || !token) return;
    setLoading(true);
    setError(null);
    try {
      const fetched = await listStravaActivities(token);
      setActivities(fetched);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load activities');
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async (activityId: number) => {
    if (!enabled || !token) return;
    setLoading(true);
    setError(null);
    try {
      const response = await importStravaActivity(token, activityId);
      onImported(response);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to import activity');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="box">
      <div className="label">Strava Import</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
        {!token ? (
          <>
            <button onClick={handleConnect} disabled={loading || !enabled} aria-label="Connect Strava account">
              {!enabled ? 'Connect Strava (Pro)' : loading ? 'Connecting...' : 'Connect Strava'}
            </button>
            {!enabled && (
              <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>
                Strava import is available for Pro users.
              </div>
            )}
          </>
        ) : (
          <>
            <button onClick={handleRefreshActivities} disabled={loading || !enabled} aria-label="Refresh Strava activities">
              {loading ? 'Loading...' : 'Refresh activities'}
            </button>
            {hasActivities && (
              <div style={{ maxHeight: 180, overflowY: 'auto', border: 'var(--border)', padding: 'var(--space-2)' }}>
                {activities.map((activity) => (
                  <div
                    key={activity.id}
                    style={{
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      padding: 'var(--space-2) 0',
                      borderBottom: '1px solid #eee',
                    }}
                  >
                    <div style={{ fontSize: 'var(--text-xs)' }}>
                      <div>{activity.name}</div>
                      <div style={{ color: 'var(--gray)' }}>
                        {(activity.distance_m / 1000).toFixed(1)} km
                      </div>
                    </div>
                    <button
                      onClick={() => handleImport(activity.id)}
                      style={{ padding: 'var(--space-1) var(--space-2)', fontSize: 'var(--text-xs)' }}
                    >
                      Import
                    </button>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
        {error && (
          <div style={{ fontSize: 'var(--text-xs)', color: '#c00' }} aria-live="polite">
            {error}
          </div>
        )}
      </div>
    </div>
  );
}
