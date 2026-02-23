import { describe, expect, it } from 'vitest';
import { drawRideVizWatermark } from './watermark';

function createMockCtx() {
  const calls: string[] = [];
  const record =
    (name: string) =>
    (..._args: unknown[]) => {
      calls.push(name);
    };

  const ctx: Partial<CanvasRenderingContext2D> = {
    save: record('save') as CanvasRenderingContext2D['save'],
    restore: record('restore') as CanvasRenderingContext2D['restore'],
    beginPath: record('beginPath') as CanvasRenderingContext2D['beginPath'],
    moveTo: record('moveTo') as CanvasRenderingContext2D['moveTo'],
    arcTo: record('arcTo') as CanvasRenderingContext2D['arcTo'],
    closePath: record('closePath') as CanvasRenderingContext2D['closePath'],
    fill: record('fill') as CanvasRenderingContext2D['fill'],
    stroke: record('stroke') as CanvasRenderingContext2D['stroke'],
    strokeText: record('strokeText') as CanvasRenderingContext2D['strokeText'],
    fillText: record('fillText') as CanvasRenderingContext2D['fillText'],
    measureText: ((text: string) => ({ width: text.length * 10 }) as TextMetrics) as CanvasRenderingContext2D['measureText'],
  };

  return { ctx: ctx as CanvasRenderingContext2D, calls };
}

describe('drawRideVizWatermark', () => {
  it('draws a pill and outlined text', () => {
    const { ctx, calls } = createMockCtx();
    drawRideVizWatermark(ctx, 800, 600);

    expect(calls).toContain('fill');
    expect(calls).toContain('stroke');
    expect(calls).toContain('fillText');
  });

  it('does not throw on tiny canvases', () => {
    const { ctx } = createMockCtx();
    expect(() => drawRideVizWatermark(ctx, 8, 8)).not.toThrow();
  });
});
