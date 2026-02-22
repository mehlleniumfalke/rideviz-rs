import { useEffect, useState } from 'react';
import posthog from 'posthog-js';
import ToolPage from './pages/ToolPage/ToolPageImpl';

type RoutePath = '/' | '/app';

function getRoutePath(): RoutePath {
  return window.location.pathname === '/' ? '/' : '/app';
}

function App() {
  const [routePath, setRoutePath] = useState<RoutePath>(getRoutePath());

  useEffect(() => {
    posthog.capture('$pageview', { path: routePath });
  }, [routePath]);

  useEffect(() => {
    function onPopState() {
      setRoutePath(getRoutePath());
    }
    window.addEventListener('popstate', onPopState);
    return () => window.removeEventListener('popstate', onPopState);
  }, []);

  function navigate(path: RoutePath) {
    if (window.location.pathname === path) return;
    window.history.pushState({}, '', path);
    setRoutePath(path);
  }

  if (routePath === '/') {
    return (
      <div
        style={{
          minHeight: '100vh',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 'var(--space-8)',
          padding: 'var(--space-4)',
        }}
      >
        <div style={{ textAlign: 'center' }}>
          <h1 style={{ fontSize: 'var(--text-2xl)', fontWeight: 600, marginBottom: 'var(--space-2)' }}>
            rideviz.online
          </h1>
          <p style={{ color: 'var(--gray)', fontSize: 'var(--text-base)' }}>
            Your ride. Visualized.
          </p>
        </div>

        {/* Sample animation preview */}
        <div
          style={{
            border: 'var(--border)',
            padding: '2px',
            background: '#000',
            maxWidth: '600px',
            width: '100%',
          }}
        >
          <img
            src="/rideviz-route.apng"
            alt="Sample route animation"
            style={{
              display: 'block',
              width: '100%',
              height: 'auto',
            }}
          />
        </div>

        <button
          onClick={() => navigate('/app')}
          style={{ padding: 'var(--space-3) var(--space-6)', fontSize: 'var(--text-base)' }}
        >
          Start →
        </button>
      </div>
    );
  }

  return <ToolPage onNavigateHome={() => navigate('/')} />;
}

export default App;
