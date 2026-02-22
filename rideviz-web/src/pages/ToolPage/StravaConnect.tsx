import { useEffect, useState } from 'react';
import {
  completeStravaAuth,
  getStravaAuthUrl,
  importStravaActivity,
  listStravaActivities,
} from '../../api/client';
import type { StravaActivitySummary, StravaByoCredentials, UploadResponse } from '../../types/api';

const PAGE_SIZE = 100;
const STRAVA_ORANGE = '#FC5200';

interface StravaConnectProps {
  onImported: (response: UploadResponse) => void;
  enabled: boolean;
  licenseToken: string | null;
}

const STORAGE_KEY_STRAVA_TOKEN = 'rideviz_strava_token';
const STORAGE_KEY_STRAVA_CREDENTIALS = 'rideviz_strava_byo_config';

function loadSavedCredentials(): StravaByoCredentials | null {
  const raw = localStorage.getItem(STORAGE_KEY_STRAVA_CREDENTIALS);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<StravaByoCredentials>;
    const clientId = parsed.client_id?.trim();
    const clientSecret = parsed.client_secret?.trim();
    if (!clientId || !clientSecret) return null;
    return { client_id: clientId, client_secret: clientSecret };
  } catch {
    return null;
  }
}

export default function StravaConnect({ onImported, enabled, licenseToken }: StravaConnectProps) {
  const [savedCredentials] = useState<StravaByoCredentials | null>(() => loadSavedCredentials());
  const [loading, setLoading] = useState(false);
  const [isTouchDevice, setIsTouchDevice] = useState(false);
  const [token, setToken] = useState<string | null>(localStorage.getItem(STORAGE_KEY_STRAVA_TOKEN));
  const [clientId, setClientId] = useState(savedCredentials?.client_id ?? '');
  const [clientSecret, setClientSecret] = useState(savedCredentials?.client_secret ?? '');
  const [activities, setActivities] = useState<StravaActivitySummary[]>([]);
  const [page, setPage] = useState(1);
  const [hasMore, setHasMore] = useState(true);
  const [search, setSearch] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(
    savedCredentials ? 'Personal Strava app credentials are saved on this device.' : null,
  );
  const [showGuide, setShowGuide] = useState(false);

  useEffect(() => {
    const media = window.matchMedia('(pointer: coarse)');
    const sync = () => setIsTouchDevice(media.matches);
    sync();
    media.addEventListener('change', sync);
    return () => media.removeEventListener('change', sync);
  }, []);

  const resetConnection = (message?: string) => {
    localStorage.removeItem(STORAGE_KEY_STRAVA_TOKEN);
    setToken(null);
    setActivities([]);
    setPage(1);
    setHasMore(true);
    setSearch('');
    setError(message ?? null);
  };

  const readCredentials = (): StravaByoCredentials | undefined => {
    const normalizedClientId = clientId.trim();
    const normalizedClientSecret = clientSecret.trim();
    if (!normalizedClientId && !normalizedClientSecret) {
      return undefined;
    }
    if (!normalizedClientId || !normalizedClientSecret) {
      throw new Error('Provide both Strava Client ID and Client Secret.');
    }
    return {
      client_id: normalizedClientId,
      client_secret: normalizedClientSecret,
    };
  };

  const handleSaveCredentials = () => {
    try {
      const credentials = readCredentials();
      if (!credentials) {
        localStorage.removeItem(STORAGE_KEY_STRAVA_CREDENTIALS);
        setStatus('Saved credentials cleared. RideViz default Strava app will be used.');
      } else {
        localStorage.setItem(STORAGE_KEY_STRAVA_CREDENTIALS, JSON.stringify(credentials));
        setStatus('Personal Strava app credentials saved on this device.');
      }
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save Strava credentials.');
      setStatus(null);
    }
  };

  const handleClearCredentials = () => {
    setClientId('');
    setClientSecret('');
    localStorage.removeItem(STORAGE_KEY_STRAVA_CREDENTIALS);
    setStatus('Saved credentials cleared. RideViz default Strava app will be used.');
    setError(null);
  };

  const isSessionError = (message: string) => {
    const lower = message.toLowerCase();
    return (
      lower.includes('expired or unknown strava session') ||
      lower.includes('missing strava bearer token') ||
      lower.includes('unauthorized') ||
      lower.includes('401')
    );
  };

  const handleActionError = (err: unknown, fallback: string) => {
    const message = err instanceof Error ? err.message : fallback;
    if (isSessionError(message)) {
      resetConnection('Strava session expired. Please reconnect.');
      return;
    }
    setError(message);
    setStatus(null);
  };

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
        if (!licenseToken) {
          throw new Error('Pro license required. Please verify your license and try again.');
        }
        const callback = await completeStravaAuth(code, state, licenseToken);
        if (cancelled) return;
        localStorage.setItem(STORAGE_KEY_STRAVA_TOKEN, callback.access_token);
        setToken(callback.access_token);
        const fetched = await listStravaActivities(callback.access_token, 1);
        if (!cancelled) {
          setActivities(fetched);
          setPage(1);
          setHasMore(fetched.length === PAGE_SIZE);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to connect Strava');
          setStatus(null);
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
  }, [enabled, licenseToken]);

  const handleConnect = async () => {
    if (!enabled) return;
    if (!licenseToken) {
      setError('Pro license required. Please verify your license and try again.');
      setStatus(null);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const credentials = readCredentials();
      const auth = await getStravaAuthUrl(licenseToken, credentials);
      window.location.assign(auth.auth_url);
    } catch (err) {
      handleActionError(err, 'Failed to connect Strava');
      setLoading(false);
    }
  };

  const handleLoadActivities = async (tok: string, nextPage: number, append: boolean) => {
    setLoading(true);
    setError(null);
    try {
      const fetched = await listStravaActivities(tok, nextPage);
      setActivities((prev) => (append ? [...prev, ...fetched] : fetched));
      setPage(nextPage);
      setHasMore(fetched.length === PAGE_SIZE);
    } catch (err) {
      handleActionError(err, 'Failed to load activities');
    } finally {
      setLoading(false);
    }
  };

  const handleLoadMore = () => {
    if (!token) return;
    void handleLoadActivities(token, page + 1, true);
  };

  const handleImport = async (activityId: number) => {
    if (!enabled || !token) return;
    setLoading(true);
    setError(null);
    try {
      const response = await importStravaActivity(token, activityId);
      onImported(response);
    } catch (err) {
      handleActionError(err, 'Failed to import activity');
    } finally {
      setLoading(false);
    }
  };

  const filtered = search.trim()
    ? activities.filter((a) => a.name.toLowerCase().includes(search.trim().toLowerCase()))
    : activities;
  const credentialInputStyle = {
    width: '100%',
    minHeight: 44,
    border: 'var(--border)',
    padding: '0 var(--space-2)',
    fontSize: 'var(--text-sm)',
    fontFamily: 'var(--font-body)',
  } as const;

  return (
    <div className="box">
      <div className="label">Strava Import</div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-2)' }}>
        {!token ? (
          <>
            {enabled && (
              <div style={{ display: 'grid', gap: 'var(--space-2)' }}>
                <label style={{ display: 'grid', gap: 'var(--space-1)' }}>
                  <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>Client ID</span>
                  <input
                    type="text"
                    placeholder="Strava Client ID"
                    value={clientId}
                    onChange={(event) => setClientId(event.target.value)}
                    style={credentialInputStyle}
                  />
                </label>
                <label style={{ display: 'grid', gap: 'var(--space-1)' }}>
                  <span style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>Client Secret</span>
                  <input
                    type="password"
                    placeholder="Strava Client Secret"
                    value={clientSecret}
                    onChange={(event) => setClientSecret(event.target.value)}
                    style={credentialInputStyle}
                  />
                </label>
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-2)' }}>
                  <button onClick={handleSaveCredentials} disabled={loading}>
                    Save keys
                  </button>
                  <button onClick={handleClearCredentials} disabled={loading}>
                    Clear keys
                  </button>
                </div>
                <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>
                  Your Client Secret is sensitive. It is stored only on this device.
                </div>
                <button
                  onClick={() => setShowGuide((prev) => !prev)}
                  disabled={loading}
                  style={{ fontSize: 'var(--text-xs)' }}
                >
                  {showGuide ? 'Hide setup guide' : 'How to get Strava keys'}
                </button>
                {showGuide && (
                  <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)', border: 'var(--border)', padding: 'var(--space-2)' }}>
                    <ol style={{ margin: 0, paddingInlineStart: 'var(--space-4)', display: 'grid', gap: 'var(--space-1)' }}>
                      <li>
                        Open <a href="https://www.strava.com/settings/api" target="_blank" rel="noopener noreferrer">Strava API Settings</a>.
                      </li>
                      <li>Create a new API application.</li>
                      <li>
                        Set <strong>Authorization Callback Domain</strong> to{' '}
                        <span
                          style={{
                            display: 'inline-block',
                            padding: '1px 6px',
                            borderRadius: 4,
                            background: '#fff1ea',
                            border: '1px solid #ffd6c3',
                            color: STRAVA_ORANGE,
                            fontWeight: 700,
                            fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
                          }}
                        >
                          rideviz.online
                        </span>
                        .
                      </li>
                      <li>Save the Strava application settings.</li>
                      <li>Copy Client ID and Client Secret from Strava.</li>
                      <li>Paste both keys above, click Save keys, then Connect with Strava.</li>
                    </ol>
                  </div>
                )}
              </div>
            )}
            <button
              onClick={handleConnect}
              disabled={loading || !enabled}
              aria-label="Connect with Strava"
              style={{
                background: 'none',
                border: 'none',
                padding: 0,
                cursor: enabled ? 'pointer' : 'not-allowed',
                opacity: loading || !enabled ? 0.5 : 1,
                display: 'flex',
                justifyContent: 'center',
              }}
            >
              <img
                src="/btn_strava_connect_with_white.png"
                alt={loading ? 'Connecting...' : 'Connect with Strava'}
                style={{ display: 'block', height: 48, width: 'auto' }}
              />
            </button>
            {!enabled && (
              <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>
                Strava BYO import is available for Pro users.
              </div>
            )}
          </>
        ) : (
          <>
            {activities.length === 0 && !loading && (
              <button
                onClick={() => token && void handleLoadActivities(token, 1, false)}
                disabled={loading || !enabled}
                aria-label="Load Strava activities"
                style={{
                  height: 48,
                  background: STRAVA_ORANGE,
                  color: 'white',
                  border: 'none',
                  fontWeight: 600,
                  fontSize: 'var(--text-sm)',
                  opacity: loading ? 0.7 : 1,
                }}
              >
                Load activities
              </button>
            )}
            {activities.length > 0 && (
              <input
                type="search"
                placeholder="Search activities..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                style={{ width: '100%', boxSizing: 'border-box', padding: 'var(--space-2)', fontSize: 'var(--text-sm)' }}
              />
            )}
            {loading && activities.length === 0 && (
              <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)', textAlign: 'center', padding: 'var(--space-2)', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', gap: 'var(--space-2)', width: '100%' }}>
                <span className="spinner" aria-hidden />
                <span>Loading activities...</span>
              </div>
            )}
            {filtered.length > 0 && (
              <div style={{ maxHeight: isTouchDevice ? 360 : 240, overflowY: 'auto', border: 'var(--border)' }}>
                {filtered.map((activity) => (
                  <button
                    key={activity.id}
                    onClick={() => handleImport(activity.id)}
                    disabled={loading}
                    aria-label={`Import ${activity.name}`}
                    style={{
                      width: '100%',
                      display: 'flex',
                      justifyContent: 'space-between',
                      alignItems: 'center',
                      gap: 'var(--space-3)',
                      padding: 'var(--space-3)',
                      borderBottom: '1px solid #eee',
                      borderTop: 'none',
                      borderLeft: 'none',
                      borderRight: 'none',
                      background: 'var(--white)',
                      color: 'var(--black)',
                      textAlign: 'left',
                      fontFamily: 'var(--font-body)',
                    }}
                  >
                    <span style={{ fontSize: 'var(--text-sm)' }}>
                      <span>{activity.name}</span>
                      <span style={{ color: 'var(--gray)', display: 'block', marginTop: 2 }}>
                        {(activity.distance_m / 1000).toFixed(1)} km
                      </span>
                    </span>
                    <span style={{ fontSize: 'var(--text-xs)', color: STRAVA_ORANGE, fontWeight: 600 }}>
                      Import →
                    </span>
                  </button>
                ))}
                {!search && hasMore && (
                  <div style={{ padding: 'var(--space-2)' }}>
                    <button
                      onClick={handleLoadMore}
                      disabled={loading}
                      style={{ fontSize: 'var(--text-xs)', width: '100%' }}
                    >
                      {loading ? 'Loading...' : 'Load more'}
                    </button>
                  </div>
                )}
              </div>
            )}
            {filtered.length > 0 && isTouchDevice && (
              <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>
                Tap an activity row to import.
              </div>
            )}
            {search && filtered.length === 0 && (
              <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }}>
                No activities match "{search}"
              </div>
            )}
            <button onClick={() => resetConnection()} disabled={loading} aria-label="Disconnect Strava">
              Disconnect Strava
            </button>
            <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)', textAlign: 'center' }}>
              Powered by{' '}
              <a
                href="https://www.strava.com"
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: STRAVA_ORANGE, fontWeight: 600, textDecoration: 'none' }}
              >
                Strava
              </a>
            </div>
          </>
        )}
        {status && (
          <div style={{ fontSize: 'var(--text-xs)', color: 'var(--gray)' }} aria-live="polite">
            {status}
          </div>
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
