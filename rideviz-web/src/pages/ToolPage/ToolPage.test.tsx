import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import ToolPage from './ToolPageImpl';

const uploadFileMock = vi.fn();
const getRouteDataMock = vi.fn();
const getVisualizationMock = vi.fn();

vi.mock('posthog-js', () => ({
  default: {
    capture: vi.fn(),
  },
}));

vi.mock('../../api/client', () => ({
  uploadFile: (...args: unknown[]) => uploadFileMock(...args),
  getRouteData: (...args: unknown[]) => getRouteDataMock(...args),
  getVisualization: (...args: unknown[]) => getVisualizationMock(...args),
}));

describe('ToolPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    uploadFileMock.mockResolvedValue({
      file_id: 'file-123',
      file_type: 'gpx',
      metrics: {
        distance_km: 10,
        elevation_gain_m: 123,
        duration_seconds: 1800,
        avg_speed_kmh: 20,
        avg_heart_rate: 150,
        max_heart_rate: 170,
        avg_power: 210,
        max_power: 450,
      },
      available_data: {
        has_coordinates: true,
        has_elevation: true,
        has_heart_rate: true,
        has_power: true,
      },
    });
    getRouteDataMock.mockResolvedValue({
      file_id: 'file-123',
      viz_data: {
        points: [
          { x: 0, y: 0, value: 0.1, elevation: 10 },
          { x: 1, y: 1, value: 0.9, elevation: 30 },
        ],
      },
      metrics: {},
      available_data: {},
    });
    getVisualizationMock.mockResolvedValue(new Blob(['png'], { type: 'image/png' }));
  });

  it('handles upload and triggers static download request', async () => {
    render(<ToolPage onNavigateHome={vi.fn()} />);

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    expect(input).toBeTruthy();

    const file = new File(['<gpx></gpx>'], 'ride.gpx');
    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(screen.getByText('File loaded')).toBeInTheDocument();
    });

    const downloadButton = screen.getByRole('button', { name: /Download generated export/i });
    fireEvent.click(downloadButton);

    await waitFor(() => {
      expect(getVisualizationMock).toHaveBeenCalled();
    });

    const [payload] = getVisualizationMock.mock.calls[0] as [Record<string, unknown>];
    expect(payload).toEqual(expect.objectContaining({ file_id: 'file-123' }));
    expect(payload).not.toHaveProperty('watermark');
  });
});
