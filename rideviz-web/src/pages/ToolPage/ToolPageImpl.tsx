
import { useEffect, useMemo, useRef, useState } from 'react';
import posthog from 'posthog-js';
import {
  completeCheckoutSession,
  exportVideo,
  getRouteData,
  getVisualization,
  handleStripeWebhookCompletion,
  verifyLicense,
  uploadFile,
} from '../../api/client';
import { buildAnimationProgress } from '../../engine/animate';
import { renderFrame } from '../../engine/render';
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

const getStoredDuration = () => Number(localStorage.getItem(STORAGE_KEY_DURATION) ?? 9);
const getStoredFps = () => Number(localStorage.getItem(STORAGE_KEY_FPS) ?? 30);

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

  const previewCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const abortControllerRef = useRef<AbortController | null>(null);
  const statsEntries = useMemo(() => buildStatsEntries(config.stats, metrics, availableData), [config.stats, metrics, availableData]);
  const selectedFormat = useMemo(() => getExportFormat(config.exportPreset), [config.exportPreset]);

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
  };

  const handleFileSelect = async (file: File) => {
    setIsUploading(true);
    setUploadError(null);
    try {
      handleImportedActivity(await uploadFile(file));
    } catch (error) {
      setUploadError(error instanceof Error ? error.message : 'Upload failed');
    } finally {
      setIsUploading(false);
    }
  };

  useEffect(() => {
    if (!fileId) return;
    if (availableData && !availableData.has_elevation) {
      setPreviewError('3D animation requires elevation data.');
      return;
    }
    abortControllerRef.current?.abort();
    const controller = new AbortController();
    abortControllerRef.current = controller;
    setIsLoadingPreview(true);
    setPreviewError(null);
    getRouteData(fileId, { colorBy: config.colorBy ?? undefined, smoothing: config.smoothing })
      .then((response) => !controller.signal.aborted && setRouteData(response.viz_data))
      .catch((error) => !controller.signal.aborted && setPreviewError(error instanceof Error ? error.message : 'Failed to load route data'))
      .finally(() => !controller.signal.aborted && setIsLoadingPreview(false));
    return () => controller.abort();
  }, [fileId, availableData, config.colorBy, config.smoothing]);

  useEffect(() => {
    if (config.gradient === 'black' && config.colorBy !== null) {
      setConfig((prev) => ({ ...prev, colorBy: null }));
    }
  }, [config.gradient, config.colorBy]);

  useEffect(() => {
    const canvas = previewCanvasRef.current;
    if (!canvas || !routeData || !fileId) return;
    canvas.width = selectedFormat.width;
    canvas.height = selectedFormat.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    let raf = 0;
    const draw = (progress: number) => {
      renderFrame(ctx, { data: routeData, stats: statsEntries, options: { width: selectedFormat.width, height: selectedFormat.height, padding: FIXED_PADDING, strokeWidth: FIXED_STROKE_WIDTH, smoothing: config.smoothing, glow: config.glow, background: config.background, gradient: config.gradient, progress } });
      if (!hasProAccess && !config.animated) {
        drawWatermark(ctx, selectedFormat.width, selectedFormat.height);
      }
    };
    if (!config.animated) {
      draw(1);
      return;
    }
    const start = performance.now();
    const durationMs = Math.max(1000, config.duration * 1000);
    const loop = (now: number) => {
      const linear = ((now - start) % durationMs) / durationMs;
      draw(buildAnimationProgress(Math.round(linear * 1000), 1001));
      raf = window.requestAnimationFrame(loop);
    };
    raf = window.requestAnimationFrame(loop);
    return () => window.cancelAnimationFrame(raf);
  }, [fileId, routeData, config, statsEntries, selectedFormat, hasProAccess]);

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
      } catch (error) {
        if (cancelled) return;
        console.error('License verification failed:', error);
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
          const result = await handleStripeWebhookCompletion({
            type: 'checkout.session.completed',
            data: {
              object: {
                customer_email: email,
              },
            },
          });
          if (cancelled) return;
          setLicenseToken(result.token);
          posthog.capture('license_issued_mock', { email });
        } else if (checkout === 'success' && sessionId && !licenseToken) {
          const result = await completeCheckoutSession(sessionId);
          if (cancelled) return;
          setLicenseToken(result.token);
          posthog.capture('license_issued_checkout', { sessionId });
        } else if (checkout === 'cancel') {
          posthog.capture('checkout_cancelled');
        }

        if (cancelled) return;
        clearCheckoutParams();
      } catch (error) {
        if (!cancelled) {
          console.error('Checkout completion failed:', error);
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
      watermark: !hasProAccess,
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

  const drawWatermark = (ctx: CanvasRenderingContext2D, width: number, height: number) => {
    const fontSize = Math.max(13, Math.round(height * 0.02));
    ctx.save();
    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    ctx.fillStyle = 'rgb(0,0,0)';
    ctx.font = `${fontSize}px Geist Pixel, monospace`;
    ctx.fillText('created with rideviz.online', width / 2, height - 16);
    ctx.restore();
  };

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
    drawWatermark(ctx, canvas.width, canvas.height);
    return canvasToPngBlob(canvas);
  };

  const buildVideoRequest = (): VideoExportRequest | null => {
    if (!fileId) return null;
    if (config.background === 'transparent') {
      throw new Error('MP4 export requires a black or white background.');
    }
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
      duration_seconds: config.duration,
      fps: config.fps,
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

  const handleDownload = async () => {
    if (!fileId) return;
    setIsExporting(true);
    setShareStatus(null);
    try {
      if (config.animated) {
        const { blob, fileName } = await buildVideoBlob();
        downloadBlob(blob, fileName);
      } else {
        const blob = await buildStaticBlob();
        downloadBlob(blob, 'rideviz-route.png');
      }
      persistHistory();
      posthog.capture('download_export', { animated: config.animated, exportPreset: config.exportPreset });
    } catch (error) {
      console.error('Download failed:', error);
      setShareStatus(getExportErrorMessage(error));
    } finally {
      setIsExporting(false);
    }
  };

  const handleShare = async () => {
    if (!fileId) return;
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
        posthog.capture('share_export_fallback', { copied, animated: config.animated });
        return;
      }

      let blob: Blob;
      let fileName: string;
      if (config.animated) {
        ({ blob, fileName } = await buildVideoBlob());
      } else {
        blob = await buildStaticBlob();
        fileName = 'rideviz-route.png';
      }
      await navigator.share({ title: 'RideViz export', text: 'Made with rideviz.online', files: [new File([blob], fileName, { type: blob.type || 'application/octet-stream' })] });
      persistHistory();
      setShareStatus('Shared successfully.');
      posthog.capture('share_export', { animated: config.animated });
    } catch (error) {
      console.error('Share failed:', error);
      setShareStatus(getExportErrorMessage(error));
    } finally {
      setIsExporting(false);
    }
  };

  const handleReset = () => {
    abortControllerRef.current?.abort();
    setFileId(null);
    setRouteData(null);
    setAvailableData(null);
    setMetrics(null);
    setShareStatus(null);
    setUploadError(null);
    setPreviewError(null);
  };

  return (
    <div style={{ minHeight: '100vh', padding: 'var(--space-4)' }}>
      <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 'var(--space-4)', paddingBottom: 'var(--space-4)', borderBottom: 'var(--border)' }}>
        <h1 style={{ fontSize: 'var(--text-xl)', fontWeight: 600 }}>RideViz</h1>
        <button onClick={onNavigateHome} aria-label="Back to home">← Back</button>
      </header>
      <div className="tool-layout" style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 'var(--space-4)', minHeight: 'calc(100vh - 100px)' }}>
        <PreviewPanel canvasRef={previewCanvasRef} previewWidth={selectedFormat.width} previewHeight={selectedFormat.height} isLoading={isLoadingPreview} error={previewError} isExporting={isExporting} onDownload={handleDownload} onShare={handleShare} fileId={fileId} background={config.background} canShare={Boolean(navigator.share)} isAnimated={config.animated} canAnimatedExport={hasProAccess} shareStatus={shareStatus} />
        <aside className="tool-controls" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
          {!fileId ? (
            <>
              <UploadZone onFileSelect={handleFileSelect} isUploading={isUploading} error={uploadError || undefined} />
              <StravaConnect onImported={handleImportedActivity} enabled={hasProAccess} />
              <UpgradePanel onLicenseToken={setLicenseToken} currentToken={licenseToken} />
            </>
          ) : (
            <div className="box" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: 'var(--text-sm)', color: 'var(--gray)' }}>File loaded</span>
              <button onClick={handleReset} style={{ padding: 'var(--space-1) var(--space-2)', fontSize: 'var(--text-xs)' }}>Reset</button>
            </div>
          )}
          {fileId && (
            <>
              <GradientPicker selectedGradient={config.gradient} onChange={(gradient) => setConfig({ ...config, gradient })} />
              {availableData && <ColorByPicker value={config.colorBy} gradient={config.gradient} availableData={availableData} onChange={(colorBy) => setConfig({ ...config, colorBy })} />}
              <BackgroundPicker value={config.background} onChange={(background) => setConfig({ ...config, background })} />
              {availableData && metrics && <StatsPicker value={config.stats} availableData={availableData} metrics={metrics} onChange={(stats) => setConfig({ ...config, stats })} />}
              <ExportFormatPicker value={config.exportPreset} onChange={(exportPreset) => setConfig({ ...config, exportPreset })} />
              {config.animated && <DurationControl duration={config.duration} fps={config.fps} width={selectedFormat.width} height={selectedFormat.height} onChange={(updates) => setConfig({ ...config, ...updates })} />}
              <AdvancedPanel smoothing={config.smoothing} glow={config.glow} animated={config.animated} onChange={(updates) => setConfig({ ...config, ...updates })} />
              <UpgradePanel onLicenseToken={setLicenseToken} currentToken={licenseToken} />
            </>
          )}
        </aside>
      </div>
      <style>{`@media (max-width: 768px) { .tool-layout { grid-template-columns: 1fr !important; min-height: unset !important; } .tool-controls { order: 1; } .preview-panel { order: 2; min-height: 280px !important; } }`}</style>
    </div>
  );
}
