import { useState, useEffect, useRef } from 'react';
import { uploadFile, getVisualization } from '../../api/client';
import type {
  AvailableData,
  ColorByMetric,
  GradientName,
  BackgroundColor,
  ExportPreset,
  Metrics,
  StatKey,
} from '../../types/api';

import UploadZone from './UploadZone';
import GradientPicker from './GradientPicker';
import AdvancedPanel from './AdvancedPanel';
import PreviewPanel from './PreviewPanel';
import ColorByPicker from './ColorByPicker';
import BackgroundPicker from './BackgroundPicker';
import DurationControl from './DurationControl';
import ExportFormatPicker, { getExportFormat } from './ExportFormatPicker';
import StatsPicker, { isStatAvailable } from './StatsPicker';

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

function getStoredDuration(): number {
  const stored = localStorage.getItem(STORAGE_KEY_DURATION);
  return stored ? Number(stored) : 9;
}

function getStoredFps(): number {
  const stored = localStorage.getItem(STORAGE_KEY_FPS);
  return stored ? Number(stored) : 30;
}


export default function ToolPage({ onNavigateHome }: ToolPageProps) {
  const [fileId, setFileId] = useState<string | null>(null);
  const [availableData, setAvailableData] = useState<AvailableData | null>(null);
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);

  const [config, setConfig] = useState<VizConfig>({
    gradient: 'black',
    exportPreset: 'hd_landscape_16x9',
    colorBy: 'heartrate',
    strokeWidth: 3,
    padding: 40,
    smoothing: 30,
    glow: false,
    background: 'white',
    animated: false,
    duration: getStoredDuration(),
    fps: getStoredFps(),
    stats: [],
  });

  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);

  const abortControllerRef = useRef<AbortController | null>(null);

  const handleFileSelect = async (file: File) => {
    setIsUploading(true);
    setUploadError(null);

    try {
      const response = await uploadFile(file);
      setFileId(response.file_id);
      const inferredAvailableData: AvailableData = response.available_data;
      const inferredMetrics: Metrics = response.metrics;
      setAvailableData(inferredAvailableData);
      setMetrics(inferredMetrics);
      
      // Only update colorBy if heartrate is not available and we had heartrate selected
      setConfig((prev) => {
        const filteredStats = prev.stats.filter((stat) =>
          isStatAvailable(stat, inferredAvailableData, inferredMetrics),
        );
        if (prev.colorBy === 'heartrate' && !inferredAvailableData.has_heart_rate) {
          return {
            ...prev,
            colorBy: inferredAvailableData.has_elevation ? 'elevation' : 'speed',
            stats: filteredStats,
          };
        }
        return { ...prev, stats: filteredStats };
      });
      
      if (!inferredAvailableData.has_elevation) {
        setUploadError('No elevation data. 3D animation requires elevation.');
      }
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

    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    const timer = setTimeout(async () => {
      setIsLoadingPreview(true);
      setPreviewError(null);

      const controller = new AbortController();
      abortControllerRef.current = controller;

      try {
        const selectedFormat = getExportFormat(config.exportPreset);
        const requestParams: any = {
          file_id: fileId,
          gradient: config.gradient,
          width: selectedFormat.width,
          height: selectedFormat.height,
          stroke_width: config.strokeWidth,
          padding: config.padding,
          smoothing: config.smoothing,
          color_by: config.colorBy ?? undefined,
          glow: config.glow,
          background: config.background,
          stats: config.stats.length > 0 ? config.stats : undefined,
        };

        if (config.animated) {
          const previewDuration = Math.min(config.duration, 10);
          requestParams.duration_seconds = previewDuration;
          requestParams.fps = Math.min(config.fps, 30);
        }

        const blob = await getVisualization(requestParams, controller.signal);

        const url = URL.createObjectURL(blob);
        setPreviewUrl((prev) => {
          if (prev) URL.revokeObjectURL(prev);
          return url;
        });
      } catch (error: unknown) {
        if (!(error instanceof DOMException && error.name === 'AbortError')) {
          setPreviewError(error instanceof Error ? error.message : 'Failed to generate preview');
        }
      } finally {
        setIsLoadingPreview(false);
      }
    }, 300);

    return () => clearTimeout(timer);
  }, [fileId, config, availableData]);

  useEffect(() => {
    return () => {
      if (previewUrl) URL.revokeObjectURL(previewUrl);
    };
  }, [previewUrl]);

  const handleDownload = async () => {
    if (!fileId) return;

    try {
      const selectedFormat = getExportFormat(config.exportPreset);
      const requestParams: any = {
        file_id: fileId,
        gradient: config.gradient,
        width: selectedFormat.width,
        height: selectedFormat.height,
        stroke_width: config.strokeWidth,
        padding: config.padding,
        smoothing: config.smoothing,
        color_by: config.colorBy ?? undefined,
        glow: config.glow,
        background: config.background,
        stats: config.stats.length > 0 ? config.stats : undefined,
      };

      if (config.animated) {
        requestParams.duration_seconds = config.duration;
        requestParams.fps = config.fps;

        localStorage.setItem(STORAGE_KEY_DURATION, String(config.duration));
        localStorage.setItem(STORAGE_KEY_FPS, String(config.fps));
      }

      const blob = await getVisualization(requestParams);

      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = config.animated ? 'rideviz-route.apng' : 'rideviz-route.png';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Download failed:', error);
    }
  };

  const handleReset = () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }
    setFileId(null);
    setAvailableData(null);
    setMetrics(null);
    setUploadError(null);
    setPreviewError(null);
    setPreviewUrl((prev) => {
      if (prev) URL.revokeObjectURL(prev);
      return null;
    });
  };

  return (
    <div style={{ minHeight: '100vh', padding: 'var(--space-4)' }}>
      {/* Header */}
      <header
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 'var(--space-4)',
          paddingBottom: 'var(--space-4)',
          borderBottom: 'var(--border)',
        }}
      >
        <h1 style={{ fontSize: 'var(--text-xl)', fontWeight: 600 }}>RideViz</h1>
        <button onClick={onNavigateHome}>← Back</button>
      </header>

      {/* Main Layout: Animation centered, controls on right */}
      <div
        className="tool-layout"
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 320px',
          gap: 'var(--space-4)',
          minHeight: 'calc(100vh - 100px)',
        }}
      >
        {/* Left: Preview (animation is the centerpiece) */}
        <PreviewPanel
          previewUrl={previewUrl}
          isLoading={isLoadingPreview}
          error={previewError}
          onDownload={handleDownload}
          fileId={fileId}
          background={config.background}
        />

        {/* Right: Controls */}
        <aside className="tool-controls" style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
          {!fileId ? (
            <UploadZone
              onFileSelect={handleFileSelect}
              isUploading={isUploading}
              error={uploadError || undefined}
            />
          ) : (
            <div className="box" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span style={{ fontSize: 'var(--text-sm)', color: 'var(--gray)' }}>File loaded</span>
              <button onClick={handleReset} style={{ padding: 'var(--space-1) var(--space-2)', fontSize: 'var(--text-xs)' }}>
                Reset
              </button>
            </div>
          )}

          {fileId && (
            <>
              <GradientPicker
                selectedGradient={config.gradient}
                onChange={(gradient) => setConfig({ ...config, gradient })}
              />

              {availableData && (
                <ColorByPicker
                  value={config.colorBy}
                  availableData={availableData}
                  onChange={(colorBy) => setConfig({ ...config, colorBy })}
                />
              )}

              <BackgroundPicker
                value={config.background}
                onChange={(background) => setConfig({ ...config, background })}
              />

              {availableData && metrics && (
                <StatsPicker
                  value={config.stats}
                  availableData={availableData}
                  metrics={metrics}
                  onChange={(stats) => setConfig({ ...config, stats })}
                />
              )}

              <ExportFormatPicker
                value={config.exportPreset}
                onChange={(exportPreset) => setConfig({ ...config, exportPreset })}
              />

              {config.animated && (
                <DurationControl
                  duration={config.duration}
                  fps={config.fps}
                  width={getExportFormat(config.exportPreset).width}
                  height={getExportFormat(config.exportPreset).height}
                  onChange={(updates) => setConfig({ ...config, ...updates })}
                />
              )}

              <AdvancedPanel
                strokeWidth={config.strokeWidth}
                padding={config.padding}
                smoothing={config.smoothing}
                glow={config.glow}
                animated={config.animated}
                onChange={(updates) => setConfig({ ...config, ...updates })}
              />
            </>
          )}
        </aside>
      </div>

      <style>{`
        @media (max-width: 768px) {
          .tool-layout {
            grid-template-columns: 1fr !important;
            min-height: unset !important;
          }
          .tool-controls {
            order: 1;
          }
          .preview-panel {
            order: 2;
            min-height: 280px !important;
          }
        }
      `}</style>
    </div>
  );
}
