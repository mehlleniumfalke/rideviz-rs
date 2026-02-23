
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  completeCheckoutSession,
  exportVideo,
  getRouteData,
  getVisualization,
  issueMockLicense,
  verifyLicense,
  uploadFile,
} from '../../api/client';
import { captureEvent } from '../../analytics/posthog';
import { mapLinearProgressToRoute } from '../../engine/animate';
import { renderFrame } from '../../engine/render';
import { drawRideVizWatermark } from '../../engine/watermark';
import { addGenerationHistoryEntry } from '../../storage/history';
import type { AvailableData, BackgroundColor, ColorByMetric, ExportPreset, GradientName, Metrics, StatKey, UploadResponse, VideoExportRequest, VisualizeRequest, VizData } from '../../types/api';
import AdvancedPanel from './AdvancedPanel';
import BackgroundPicker from './BackgroundPicker';
import ColorByPicker from './ColorByPicker';
import DurationControl from './DurationControl';
import ExportFormatPicker from './ExportFormatPicker';
import { getExportFormat } from './exportFormats';
import GradientPicker from './GradientPicker';
import PreviewPanel from './PreviewPanel';
import StatsPicker from './StatsPicker';
import { isStatAvailable } from './statsOptions';
import { buildStatsEntries } from './statsOverlay';
import StravaConnect from './StravaConnect';
import UpgradePanel from './UpgradePanel';
import UploadZone from './UploadZone';

interface ToolPageProps {
  onNavigateHome: () => void;
}

interface VizConfig {
  gradient: GradientName;
  exportPreset: ExportPreset;
  colorBy: ColorByMetric | null;
  strokeWidth: number;
  padding: number;
  smoothing: number;
  glow: boolean;
  background: BackgroundColor;
  animated: boolean;
  duration: number;
  fps: number;
  stats: StatKey[];
}

const STORAGE_KEY_DURATION = 'rideviz_duration';
const STORAGE_KEY_FPS = 'rideviz_fps';
const STORAGE_KEY_LICENSE = 'rideviz_license_token';
const FIXED_STROKE_WIDTH = 3;
const FIXED_PADDING = 40;
type ControlSectionKey = 'appearance' | 'export' | 'pro';

const getStoredDuration = () => Math.min(15, Math.max(3, Number(localStorage.getItem(STORAGE_KEY_DURATION) ?? 9)));
const getStoredFps = () => Math.min(30, Math.max(24, Number(localStorage.getItem(STORAGE_KEY_FPS) ?? 30)));
const getIsCompactViewport = () => (typeof window !== 'undefined' ? window.matchMedia('(max-width: 1024px)').matches : false);

function errorToAnalyticsMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message.slice(0, 160);
  }
  return 'unknown_error';
}

export default function ToolPageImpl({ onNavigateHome }: ToolPageProps) {
  const [fileId, setFileId] = useState<string | null>(null);
  const [routeData, setRouteData] = useState<VizData | null>(null);
  const [availableData, setAvailableData] = useState<AvailableData | null>(null);
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [shareStatus, setShareStatus] = useState<string | null>(null);
  const [hasProAccess, setHasProAccess] = useState(false);
  const [licenseToken, setLicenseToken] = useState<string | null>(() => localStorage.getItem(STORAGE_KEY_LICENSE));
  const [config, setConfig] = useState<VizConfig>({
    gradient: 'black', exportPreset: 'hd_landscape_16x9', colorBy: 'heartrate', strokeWidth: FIXED_STROKE_WIDTH,
    padding: FIXED_PADDING, smoothing: 30, glow: false, background: 'white', animated: false,
    duration: getStoredDuration(), fps: getStoredFps(), stats: [],
  });
  const [isCompactLayout, setIsCompactLayout] = useState(getIsCompactViewport);
  const [collapsedSections, setCollapsedSections] = useState<Record<ControlSectionKey, boolean>>({
    appearance: false,
    export: true,
    pro: true,
  });

  const previewCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const trackedPreviewFilesRef = useRef<Set<string>>(new Set());
  const previousAnimatedRef = useRef(config.animated);
  const selectedFormat = useMemo(() => getExportFormat(config.exportPreset), [config.exportPreset]);

  useEffect(() => {
    captureEvent('rv_tool_opened', {
      has_saved_license_token: Boolean(localStorage.getItem(STORAGE_KEY_LICENSE)),
    });
  }, []);

  const handleImportedActivity = (response: UploadResponse) => {
    setFileId(response.file_id);
    setRouteData(null);
    setPreviewError(null);
    setShareStatus(null);
    setUploadError(null);
    setAvailableData(response.available_data);
    setMetrics(response.metrics);
    setConfig((prev) => {
      const filtered = prev.stats.filter((stat) => isStatAvailable(stat, response.available_data, response.metrics));
      if (prev.colorBy === 'heartrate' && !response.available_data.has_heart_rate) {
        return { ...prev, colorBy: response.available_data.has_elevation ? 'elevation' : 'speed', stats: filtered };
      }
      return { ...prev, stats: filtered };
    });
    captureEvent('rv_activity_import_succeeded', {
      file_type: response.file_type,
      has_elevation: response.available_data.has_elevation,
      has_heart_rate: response.available_data.has_heart_rate,
      has_power: response.available_data.has_power,
      distance_km: Number(response.metrics.distance_km.toFixed(2)),
      duration_seconds: response.metrics.duration_seconds,
    });
  };

  const handleFileSelect = async (file: File) => {
    setIsUploading(true);
    setUploadError(null);
    captureEvent('rv_upload_started', {
      file_extension: file.name.split('.').pop()?.toLowerCase() ?? 'unknown',
      file_size_kb: Math.round(file.size / 1024),
    });
    try {
      handleImportedActivity(await uploadFile(file));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Upload failed';
      captureEvent('rv_upload_failed', {
        error: errorToAnalyticsMessage(error),
      });
      setUploadError(message);
    } finally {
      setIsUploading(false);
    }
  };

  useEffect(() => {
    if (!fileId) return;
    if (availableData && !availableData.has_elevation) {
      setPreviewError('3D animation requires elevation data.');
      captureEvent('rv_preview_blocked', { reason: 'missing_elevation' });
      return;
    }
    abortControllerRef.current?.abort();
    const controller = new AbortController();
    abortControllerRef.current = controller;
    setIsLoadingPreview(true);
    setPreviewError(null);
    captureEvent('rv_preview_requested', {
      color_by: config.colorBy ?? 'none',
      smoothing: config.smoothing,
      has_file: true,
    });
    getRouteData(fileId, { colorBy: config.colorBy ?? undefined, smoothing: config.smoothing })
      .then((response) => {
        if (controller.signal.aborted) return;
        setRouteData(response.viz_data);
        if (!trackedPreviewFilesRef.current.has(fileId)) {
          trackedPreviewFilesRef.current.add(fileId);
          captureEvent('rv_generation_ready', {
            color_by: config.colorBy ?? 'none',
            smoothing: config.smoothing,
            point_count: response.viz_data.points.length,
          });
        }
      })
      .catch((error) => {
        if (controller.signal.aborted) return;
        captureEvent('rv_preview_failed', {
          error: errorToAnalyticsMessage(error),
          color_by: config.colorBy ?? 'none',
          smoothing: config.smoothing,
        });
        setPreviewError(error instanceof Error ? error.message : 'Failed to load route data');
      })
      .finally(() => !controller.signal.aborted && setIsLoadingPreview(false));
    return () => controller.abort();
  }, [fileId, availableData, config.colorBy, config.smoothing]);

  useEffect(() => {
    if (config.gradient === 'black' && config.colorBy !== null) {
      setConfig((prev) => ({ ...prev, colorBy: null }));
    }
  }, [config.gradient, config.colorBy]);

  useEffect(() => {
    if (previousAnimatedRef.current === config.animated) return;
    captureEvent('rv_animation_mode_changed', {
      animated: config.animated,
      has_pro_access: hasProAccess,
    });
    if (config.animated && !hasProAccess) {
      captureEvent('rv_pro_intent', {
        source: 'animated_toggle',
      });
    }
    previousAnimatedRef.current = config.animated;
  }, [config.animated, hasProAccess]);

  useEffect(() => {
    const media = window.matchMedia('(max-width: 1024px)');
    const syncLayout = () => setIsCompactLayout(media.matches);
    syncLayout();
    media.addEventListener('change', syncLayout);
    return () => media.removeEventListener('change', syncLayout);
  }, []);

  useEffect(() => {
    const canvas = previewCanvasRef.current;
    if (!canvas || !routeData || !fileId) return;
    canvas.width = selectedFormat.width;
    canvas.height = selectedFormat.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    let raf = 0;
    const draw = (progress: number) => {
      const stats = buildStatsEntries(
        config.stats,
        metrics,
        availableData,
        routeData,
        progress,
      );
      renderFrame(ctx, { data: routeData, stats, options: { width: selectedFormat.width, height: selectedFormat.height, padding: FIXED_PADDING, strokeWidth: FIXED_STROKE_WIDTH, smoothing: config.smoothing, glow: config.glow, background: config.background, gradient: config.gradient, progress } });
    };
    if (!config.animated) {
      draw(1);
      return;
    }
    const start = performance.now();
    const durationMs = Math.max(1000, config.duration * 1000);
    const loop = (now: number) => {
      const linear = ((now - start) % durationMs) / durationMs;
      draw(mapLinearProgressToRoute(routeData.points, linear));
      raf = window.requestAnimationFrame(loop);
    };
    raf = window.requestAnimationFrame(loop);
    return () => window.cancelAnimationFrame(raf);
  }, [fileId, routeData, config, selectedFormat, hasProAccess, metrics, availableData]);

  useEffect(() => {
    if (licenseToken) localStorage.setItem(STORAGE_KEY_LICENSE, licenseToken);
    else localStorage.removeItem(STORAGE_KEY_LICENSE);
  }, [licenseToken]);

  useEffect(() => {
    let cancelled = false;
    if (!licenseToken) {
      setHasProAccess(false);
      return;
    }

    const validateLicense = async () => {
      try {
        const verification = await verifyLicense(licenseToken);
        if (cancelled) return;
        setHasProAccess(Boolean(verification.pro));
        captureEvent('rv_license_verified', {
          pro: Boolean(verification.pro),
        });
      } catch (error) {
        if (cancelled) return;
        console.error('License verification failed:', error);
        captureEvent('rv_license_verify_failed', {
          error: errorToAnalyticsMessage(error),
        });
        setHasProAccess(false);
        setLicenseToken(null);
      }
    };

    void validateLicense();
    return () => {
      cancelled = true;
    };
  }, [licenseToken]);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const checkout = params.get('checkout');
    const sessionId = params.get('session_id');
    const email = params.get('email');
    if (!checkout) return;

    let cancelled = false;
    const clearCheckoutParams = () => {
      params.delete('checkout');
      params.delete('session_id');
      params.delete('email');
      const query = params.toString();
      const nextUrl = `${window.location.pathname}${query ? `?${query}` : ''}`;
      window.history.replaceState({}, '', nextUrl);
    };

    const completeCheckout = async () => {
      try {
        if (checkout === 'mock' && email && !licenseToken) {
          if (!import.meta.env.DEV) {
            clearCheckoutParams();
            return;
          }

          const result = await issueMockLicense(email);
          if (cancelled) return;
          setLicenseToken(result.token);
          captureEvent('rv_checkout_completed', {
            mode: 'mock',
            email_domain: email.includes('@') ? email.split('@')[1] : null,
          });
          captureEvent('license_issued_mock', { email_domain: email.includes('@') ? email.split('@')[1] : null });
        } else if (checkout === 'success' && sessionId && !licenseToken) {
          const result = await completeCheckoutSession(sessionId);
          if (cancelled) return;
          setLicenseToken(result.token);
          captureEvent('rv_checkout_completed', {
            mode: 'stripe',
            has_session_id: Boolean(sessionId),
          });
          captureEvent('license_issued_checkout', { has_session_id: Boolean(sessionId) });
        } else if (checkout === 'cancel') {
          captureEvent('rv_checkout_cancelled');
          captureEvent('checkout_cancelled');
        }

        if (cancelled) return;
        clearCheckoutParams();
      } catch (error) {
        if (!cancelled) {
          console.error('Checkout completion failed:', error);
          captureEvent('rv_checkout_completion_failed', {
            error: errorToAnalyticsMessage(error),
            checkout_state: checkout,
          });
          clearCheckoutParams();
        }
      }
    };

    void completeCheckout();
    return () => {
      cancelled = true;
    };
  }, [licenseToken]);

  const persistHistory = () => {
    const canvas = previewCanvasRef.current;
    if (!canvas) return;
    const thumb = document.createElement('canvas');
    thumb.width = 320;
    thumb.height = 180;
    const ctx = thumb.getContext('2d');
    if (!ctx) return;
    ctx.drawImage(canvas, 0, 0, thumb.width, thumb.height);
    addGenerationHistoryEntry({
      id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
      createdAt: new Date().toISOString(),
      thumbnailDataUrl: thumb.toDataURL('image/png'),
      config,
    });
  };

  const buildServerRequest = (): VisualizeRequest | null => {
    if (!fileId) return null;
    return {
      file_id: fileId,
      gradient: config.gradient,
      width: selectedFormat.width,
      height: selectedFormat.height,
      stroke_width: FIXED_STROKE_WIDTH,
      padding: FIXED_PADDING,
      smoothing: config.smoothing,
      color_by: config.colorBy ?? undefined,
      glow: config.glow,
      background: config.background,
      stats: config.stats.length ? config.stats : undefined,
    };
  };

  const canvasToPngBlob = (canvas: HTMLCanvasElement): Promise<Blob> =>
    new Promise((resolve, reject) => {
      canvas.toBlob((blob) => {
        if (blob) {
          resolve(blob);
          return;
        }
        reject(new Error('Failed to build PNG export.'));
      }, 'image/png');
    });

  const buildStaticBlob = async (): Promise<Blob> => {
    if (hasProAccess) {
      const request = buildServerRequest();
      if (!request) throw new Error('Missing export request.');
      return getVisualization(request, undefined, licenseToken ?? undefined);
    }

    const source = previewCanvasRef.current;
    if (!source) {
      const request = buildServerRequest();
      if (!request) throw new Error('Missing export request.');
      return getVisualization(request);
    }
    const canvas = document.createElement('canvas');
    canvas.width = source.width;
    canvas.height = source.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      const request = buildServerRequest();
      if (!request) throw new Error('Missing export request.');
      return getVisualization(request);
    }
    ctx.drawImage(source, 0, 0);
    drawRideVizWatermark(ctx, canvas.width, canvas.height);
    return canvasToPngBlob(canvas);
  };

  const buildVideoRequest = (): VideoExportRequest | null => {
    if (!fileId) return null;
    if (config.background === 'transparent') {
      throw new Error('MP4 export requires a black or white background.');
    }
    const durationSeconds = Math.min(15, Math.max(3, config.duration));
    const fps = Math.min(30, Math.max(24, config.fps));
    return {
      file_id: fileId,
      gradient: config.gradient,
      width: selectedFormat.width,
      height: selectedFormat.height,
      stroke_width: FIXED_STROKE_WIDTH,
      padding: FIXED_PADDING,
      smoothing: config.smoothing,
      color_by: config.colorBy ?? undefined,
      glow: config.glow,
      background: config.background,
      duration_seconds: durationSeconds,
      fps,
      stats: config.stats.length ? config.stats : undefined,
    };
  };

  const buildVideoBlob = async (): Promise<{ blob: Blob; fileName: string }> => {
    if (!hasProAccess || !licenseToken) {
      throw new Error('Animated MP4 export is a Pro feature. Upgrade to unlock.');
    }
    const request = buildVideoRequest();
    if (!request) throw new Error('Missing video export request.');
    localStorage.setItem(STORAGE_KEY_DURATION, String(config.duration));
    localStorage.setItem(STORAGE_KEY_FPS, String(config.fps));
    const blob = await exportVideo(request, licenseToken);
    return { blob, fileName: 'rideviz-route.mp4' };
  };

  const downloadBlob = (blob: Blob, fileName: string) => {
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = fileName;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    URL.revokeObjectURL(url);
  };

  const getExportErrorMessage = (error: unknown): string => {
    if (error instanceof DOMException && error.name === 'AbortError') {
      return 'Share cancelled.';
    }
    if (error instanceof Error && error.message.trim()) {
      return error.message;
    }
    return 'Export failed. Try a shorter duration, lower FPS, or smaller format.';
  };

  const getExportProperties = (channel: 'download' | 'share') => ({
    channel,
    animated: config.animated,
    export_preset: config.exportPreset,
    color_by: config.colorBy ?? 'none',
    background: config.background,
    gradient: config.gradient,
    has_pro_access: hasProAccess,
    stats_count: config.stats.length,
  });

  const handleDownload = async () => {
    if (!fileId) return;
    captureEvent('rv_export_started', getExportProperties('download'));
    setIsExporting(true);
    setShareStatus(null);
    try {
      let exportedFormat: 'png' | 'mp4';
      if (config.animated) {
        const { blob, fileName } = await buildVideoBlob();
        downloadBlob(blob, fileName);
        exportedFormat = 'mp4';
      } else {
        const blob = await buildStaticBlob();
        downloadBlob(blob, 'rideviz-route.png');
        exportedFormat = 'png';
      }
      persistHistory();
      captureEvent('rv_export_succeeded', {
        ...getExportProperties('download'),
        format: exportedFormat,
      });
      captureEvent('download_export', { animated: config.animated, exportPreset: config.exportPreset });
    } catch (error) {
      console.error('Download failed:', error);
      captureEvent('rv_export_failed', {
        ...getExportProperties('download'),
        error: errorToAnalyticsMessage(error),
      });
      setShareStatus(getExportErrorMessage(error));
    } finally {
      setIsExporting(false);
    }
  };

  const handleShare = async () => {
    if (!fileId) return;
    captureEvent('rv_export_started', getExportProperties('share'));
    setIsExporting(true);
    setShareStatus(null);
    try {
      if (!navigator.share) {
        const shareUrl = `${window.location.origin}/app`;
        let copied = false;
        try {
          await navigator.clipboard.writeText(shareUrl);
          copied = true;
        } catch {
          // Clipboard may be blocked in insecure contexts.
        }
        window.open(
          `https://twitter.com/intent/tweet?text=${encodeURIComponent('Made with rideviz.online')}&url=${encodeURIComponent(shareUrl)}`,
          '_blank',
          'noopener,noreferrer',
        );
        persistHistory();
        setShareStatus(copied ? 'Link copied and share options opened.' : 'Share options opened.');
        captureEvent('rv_share_fallback', {
          ...getExportProperties('share'),
          copied,
        });
        captureEvent('share_export_fallback', { copied, animated: config.animated });
        return;
      }

      let blob: Blob;
      let fileName: string;
      let exportedFormat: 'png' | 'mp4';
      if (config.animated) {
        ({ blob, fileName } = await buildVideoBlob());
        exportedFormat = 'mp4';
      } else {
        blob = await buildStaticBlob();
        fileName = 'rideviz-route.png';
        exportedFormat = 'png';
      }
      await navigator.share({ title: 'RideViz export', text: 'Made with rideviz.online', files: [new File([blob], fileName, { type: blob.type || 'application/octet-stream' })] });
      persistHistory();
      setShareStatus('Shared successfully.');
      captureEvent('rv_export_succeeded', {
        ...getExportProperties('share'),
        format: exportedFormat,
      });
      captureEvent('share_export', { animated: config.animated });
    } catch (error) {
      console.error('Share failed:', error);
      captureEvent('rv_export_failed', {
        ...getExportProperties('share'),
        error: errorToAnalyticsMessage(error),
      });
      setShareStatus(getExportErrorMessage(error));
    } finally {
      setIsExporting(false);
    }
  };

  const handleReset = () => {
    abortControllerRef.current?.abort();
    captureEvent('rv_activity_reset');
    setFileId(null);
    setRouteData(null);
    setAvailableData(null);
    setMetrics(null);
    setShareStatus(null);
    setUploadError(null);
    setPreviewError(null);
  };

  const toggleSection = (section: ControlSectionKey) => {
    setCollapsedSections((prev) => {
      const next = !prev[section];
      captureEvent('rv_control_section_toggled', { section, expanded: next });
      return { ...prev, [section]: next };
    });
  };

  const canShare = typeof navigator !== 'undefined' && typeof navigator.share === 'function';
  const showActionButtons = Boolean(fileId) && !isLoadingPreview && !previewError;
  const downloadDisabled = isExporting || (config.animated && !hasProAccess);
  const appearanceControls = (
    <>
      <GradientPicker selectedGradient={config.gradient} onChange={(gradient) => setConfig({ ...config, gradient })} />
      {availableData && <ColorByPicker value={config.colorBy} gradient={config.gradient} availableData={availableData} onChange={(colorBy) => setConfig({ ...config, colorBy })} />}
      <BackgroundPicker value={config.background} onChange={(background) => setConfig({ ...config, background })} />
      {availableData && metrics && <StatsPicker value={config.stats} availableData={availableData} metrics={metrics} onChange={(stats) => setConfig({ ...config, stats })} />}
    </>
  );
  const exportControls = (
    <>
      <ExportFormatPicker value={config.exportPreset} onChange={(exportPreset) => setConfig({ ...config, exportPreset })} />
      {config.animated && <DurationControl duration={config.duration} fps={config.fps} width={selectedFormat.width} height={selectedFormat.height} onChange={(updates) => setConfig({ ...config, ...updates })} />}
      <AdvancedPanel
        smoothing={config.smoothing}
        glow={config.glow}
        animated={config.animated}
        hasProAccess={hasProAccess}
        onChange={(updates) => setConfig({ ...config, ...updates })}
      />
    </>
  );
  const proControl = (
    <UpgradePanel
      onLicenseToken={setLicenseToken}
      currentToken={licenseToken}
      hasProAccess={hasProAccess}
    />
  );

  return (
    <div className={`tool-page${showActionButtons ? ' tool-page--with-actions' : ''}`} style={{ minHeight: '100vh', padding: 'var(--space-4)' }}>
      <header style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-3)', marginBottom: 'var(--space-4)', paddingBottom: 'var(--space-4)', borderBottom: 'var(--border)' }}>
        <button
          onClick={() => {
            captureEvent('rv_navigation', { to_path: '/' });
            onNavigateHome();
          }}
          aria-label="Back to home"
        >
          ← Back
        </button>
        <h1 style={{ fontSize: 'var(--text-xl)', fontWeight: 600 }}>RideViz</h1>
      </header>
      <div className={`tool-layout${fileId ? ' tool-layout--loaded' : ''}`} style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 320px', gap: 'var(--space-4)', minHeight: 'calc(100vh - 100px)' }}>
        <PreviewPanel
          canvasRef={previewCanvasRef}
          previewWidth={selectedFormat.width}
          previewHeight={selectedFormat.height}
          isLoading={isLoadingPreview}
          error={previewError}
          isExporting={isExporting}
          onDownload={handleDownload}
          onShare={handleShare}
          fileId={fileId}
          background={config.background}
          canShare={canShare}
          isAnimated={config.animated}
          canAnimatedExport={hasProAccess}
          shareStatus={shareStatus}
          showWatermark={!hasProAccess}
          emptyState={<UploadZone onFileSelect={handleFileSelect} isUploading={isUploading} error={uploadError || undefined} />}
        />
        <aside className="tool-controls" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
          {!fileId ? (
            <>
              <StravaConnect
                onImported={handleImportedActivity}
                enabled={hasProAccess}
                licenseToken={licenseToken}
              />
              {proControl}
            </>
          ) : (
            <div className="box" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: 'var(--text-sm)', color: 'var(--gray)' }}>File loaded</span>
              <button onClick={handleReset} style={{ padding: 'var(--space-1) var(--space-2)', fontSize: 'var(--text-xs)' }}>Reset</button>
            </div>
          )}
          {fileId && (
            isCompactLayout ? (
              <>
                <div className="box">
                  <button type="button" onClick={() => toggleSection('appearance')} aria-expanded={!collapsedSections.appearance} style={{ all: 'unset', display: 'flex', width: '100%', justifyContent: 'space-between', alignItems: 'center', cursor: 'pointer' }}>
                    <span className="label" style={{ margin: 0 }}>Appearance</span>
                    <span aria-hidden>{collapsedSections.appearance ? '▸' : '▾'}</span>
                  </button>
                </div>
                {!collapsedSections.appearance && appearanceControls}
                <div className="box">
                  <button type="button" onClick={() => toggleSection('export')} aria-expanded={!collapsedSections.export} style={{ all: 'unset', display: 'flex', width: '100%', justifyContent: 'space-between', alignItems: 'center', cursor: 'pointer' }}>
                    <span className="label" style={{ margin: 0 }}>Export</span>
                    <span aria-hidden>{collapsedSections.export ? '▸' : '▾'}</span>
                  </button>
                </div>
                {!collapsedSections.export && exportControls}
                <div className="box">
                  <button type="button" onClick={() => toggleSection('pro')} aria-expanded={!collapsedSections.pro} style={{ all: 'unset', display: 'flex', width: '100%', justifyContent: 'space-between', alignItems: 'center', cursor: 'pointer' }}>
                    <span className="label" style={{ margin: 0 }}>Pro</span>
                    <span aria-hidden>{collapsedSections.pro ? '▸' : '▾'}</span>
                  </button>
                </div>
                {!collapsedSections.pro && proControl}
              </>
            ) : (
              <>
                {appearanceControls}
                {exportControls}
                {proControl}
              </>
            )
          )}
        </aside>
      </div>
      {showActionButtons && (
        <div className="mobile-action-bar" role="region" aria-label="Export actions">
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 'var(--space-2)' }}>
            <button
              onClick={handleDownload}
              aria-label="Download generated export"
              disabled={downloadDisabled}
              style={{ width: '100%' }}
            >
              {isExporting ? 'Preparing...' : config.animated ? (hasProAccess ? 'Export MP4 ↓' : 'Upgrade for MP4') : 'Download PNG ↓'}
            </button>
            <button onClick={handleShare} aria-label="Share generated export" disabled={isExporting} style={{ width: '100%' }}>
              {canShare ? 'Share ↗' : 'Share Link ↗'}
            </button>
          </div>
          {isExporting && <div className="progress-indeterminate" style={{ marginTop: 'var(--space-2)' }} aria-hidden />}
          {shareStatus && <div style={{ marginTop: 'var(--space-2)', fontSize: 'var(--text-xs)', color: 'var(--gray)' }} aria-live="polite">{shareStatus}</div>}
        </div>
      )}
      <style>{`
        .mobile-action-bar { display: none; }
        @media (max-width: 1024px) {
          .tool-layout { grid-template-columns: 1fr !important; min-height: unset !important; }
          .tool-controls { order: 2; }
          .preview-panel { order: 1; min-height: 320px !important; }
          .tool-layout--loaded .preview-panel { order: 1; }
          .tool-layout--loaded .tool-controls { order: 2; }
          .preview-actions { display: none; }
          .mobile-action-bar {
            display: block;
            position: fixed;
            left: var(--space-4);
            right: var(--space-4);
            bottom: var(--space-4);
            z-index: 30;
            border: var(--border);
            background: var(--white);
            padding: var(--space-3);
          }
          .tool-page--with-actions {
            padding-bottom: 110px !important;
          }
        }
      `}</style>
    </div>
  );
}
