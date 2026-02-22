import { getVisualization, uploadFile } from './client';

describe('api/client', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('uploads file and returns parsed response', async () => {
    const mockResponse = {
      file_id: 'abc123',
      file_type: 'gpx',
      metrics: {
        distance_km: 10,
        elevation_gain_m: 150,
        duration_seconds: 1800,
        avg_speed_kmh: 20,
        avg_heart_rate: 140,
        max_heart_rate: 165,
        avg_power: 230,
        max_power: 400,
      },
      available_data: {
        has_coordinates: true,
        has_elevation: true,
        has_heart_rate: true,
        has_power: true,
      },
    };

    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      }),
    );

    const file = new File(['<gpx></gpx>'], 'ride.gpx');
    const result = await uploadFile(file);
    expect(result.file_id).toBe('abc123');
    expect(result.available_data.has_elevation).toBe(true);
  });

  it('requests visualization and returns blob', async () => {
    const blob = new Blob(['png'], { type: 'image/png' });
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        blob: async () => blob,
      }),
    );

    const result = await getVisualization({
      file_id: 'abc',
      gradient: 'fire',
      width: 1080,
      height: 1080,
    });
    expect(result.type).toBe('image/png');
  });
});
