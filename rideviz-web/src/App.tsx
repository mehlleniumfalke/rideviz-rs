import { useEffect, useState } from 'react';
import { captureEvent } from './analytics/posthog';
import ToolPage from './pages/ToolPage/ToolPageImpl';

type RoutePath = '/' | '/app';

const SOCIAL_PREVIEW_IMAGE = 'https://rideviz.online/assets/social-preview-1200x630.jpg?v=2';

function getRoutePath(): RoutePath {
  return window.location.pathname === '/' ? '/' : '/app';
}

function upsertMetaByName(name: string, content: string) {
  let tag = document.head.querySelector(`meta[name="${name}"]`) as HTMLMetaElement | null;
  if (!tag) {
    tag = document.createElement('meta');
    tag.setAttribute('name', name);
    document.head.appendChild(tag);
  }
  tag.setAttribute('content', content);
}

function upsertMetaByProperty(property: string, content: string) {
  let tag = document.head.querySelector(`meta[property="${property}"]`) as HTMLMetaElement | null;
  if (!tag) {
    tag = document.createElement('meta');
    tag.setAttribute('property', property);
    document.head.appendChild(tag);
  }
  tag.setAttribute('content', content);
}

function upsertCanonical(href: string) {
  let link = document.head.querySelector('link[rel="canonical"]') as HTMLLinkElement | null;
  if (!link) {
    link = document.createElement('link');
    link.setAttribute('rel', 'canonical');
    document.head.appendChild(link);
  }
  link.setAttribute('href', href);
}

function syncSeo(routePath: RoutePath) {
  const appCanonical = `${window.location.origin}/app`;
  const seo =
    routePath === '/'
      ? {
          title: 'RideViz | Open the Route Visualizer',
          description: 'Open the RideViz route visualizer to create shareable GPS route art from GPX and FIT files.',
          canonical: appCanonical,
          robots: 'noindex, follow',
        }
      : {
          title: 'RideViz Web App | Create Animated Route Art from GPX/FIT',
          description: 'Upload GPX/FIT files, customize gradients and overlays, then export static PNG or animated MP4 route visuals.',
          canonical: appCanonical,
          robots: 'index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1',
        };

  document.title = seo.title;
  upsertCanonical(seo.canonical);
  upsertMetaByName('description', seo.description);
  upsertMetaByName('robots', seo.robots);
  upsertMetaByName('googlebot', seo.robots);
  upsertMetaByName('twitter:card', 'summary_large_image');
  upsertMetaByName('twitter:title', seo.title);
  upsertMetaByName('twitter:description', seo.description);
  upsertMetaByName('twitter:image', SOCIAL_PREVIEW_IMAGE);

  upsertMetaByProperty('og:type', 'website');
  upsertMetaByProperty('og:site_name', 'RideViz');
  upsertMetaByProperty('og:title', seo.title);
  upsertMetaByProperty('og:description', seo.description);
  upsertMetaByProperty('og:url', seo.canonical);
  upsertMetaByProperty('og:image', SOCIAL_PREVIEW_IMAGE);
}

function App() {
  const [routePath, setRoutePath] = useState<RoutePath>(getRoutePath());

  useEffect(() => {
    syncSeo(routePath);
  }, [routePath]);

  useEffect(() => {
    captureEvent('$pageview', { path: routePath });
  }, [routePath]);

  useEffect(() => {
    if (routePath === '/app') {
      captureEvent('rv_app_opened', {
        referrer: document.referrer || null,
      });
    }
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
    captureEvent('rv_navigation', { to_path: path });
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
            loading="eager"
            decoding="async"
            style={{
              display: 'block',
              width: '100%',
              height: 'auto',
            }}
          />
        </div>

        <button
          onClick={() => {
            captureEvent('rv_app_cta_click', { source: 'web_home' });
            navigate('/app');
          }}
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
