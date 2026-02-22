import { getGradientStops, interpolateGradient } from './gradients';
import { prepareFramePoints } from './project';
import type { FrameRenderInput, PreparedPoint } from './types';

export function renderFrame(ctx: CanvasRenderingContext2D, input: FrameRenderInput): void {
  const { options, stats } = input;
  const points = prepareFramePoints(input.data, options);

  clearBackground(ctx, options.width, options.height, options.background);
  if (points.length < 2) return;

  drawWalls(ctx, points, options.gradient);
  drawGroundPath(ctx, points, options.strokeWidth);
  drawTopPath(ctx, points, options.strokeWidth, options.gradient, options.glow);
  drawEndpoints(ctx, points, options.gradient, options.strokeWidth);
  drawStats(ctx, stats, options);
}

function clearBackground(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  background: FrameRenderInput['options']['background'],
): void {
  ctx.clearRect(0, 0, width, height);
  if (background === 'transparent') return;
  ctx.fillStyle = background === 'black' ? '#000000' : '#FFFFFF';
  ctx.fillRect(0, 0, width, height);
}

function drawWalls(ctx: CanvasRenderingContext2D, points: PreparedPoint[], gradientName: string): void {
  const walls = points.slice(1).map((next, index) => {
    const current = points[index];
    const t = current.value ?? index / Math.max(1, points.length - 1);
    return {
      z: (current.groundY + next.groundY) * 0.5,
      color: interpolateGradient(gradientName as never, remapContrast(t)),
      current,
      next,
    };
  });
  walls.sort((a, b) => a.z - b.z);

  ctx.save();
  ctx.globalAlpha = 0.24;
  walls.forEach((wall) => {
    ctx.fillStyle = wall.color;
    ctx.beginPath();
    ctx.moveTo(wall.current.groundX, wall.current.groundY);
    ctx.lineTo(wall.current.topX, wall.current.topY);
    ctx.lineTo(wall.next.topX, wall.next.topY);
    ctx.lineTo(wall.next.groundX, wall.next.groundY);
    ctx.closePath();
    ctx.fill();
  });
  ctx.restore();
}

function drawGroundPath(ctx: CanvasRenderingContext2D, points: PreparedPoint[], strokeWidth: number): void {
  ctx.save();
  ctx.strokeStyle = 'rgba(255,255,255,0.14)';
  ctx.lineWidth = Math.max(1, strokeWidth * 0.9);
  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  ctx.beginPath();
  points.forEach((point, index) => {
    if (index === 0) ctx.moveTo(point.groundX, point.groundY);
    else ctx.lineTo(point.groundX, point.groundY);
  });
  ctx.stroke();
  ctx.restore();
}

function drawTopPath(
  ctx: CanvasRenderingContext2D,
  points: PreparedPoint[],
  strokeWidth: number,
  gradientName: string,
  glow: boolean,
): void {
  const first = points[0];
  const last = points[points.length - 1];
  const gradient = ctx.createLinearGradient(first.topX, first.topY, last.topX, last.topY);
  const stops = getGradientStops(gradientName as never);
  stops.forEach((color, index) => {
    const offset = stops.length === 1 ? 0 : index / (stops.length - 1);
    gradient.addColorStop(offset, color);
  });

  if (glow) {
    ctx.save();
    ctx.shadowColor = gradientName === 'white' ? 'rgba(255,255,255,0.8)' : 'rgba(255,255,255,0.4)';
    ctx.shadowBlur = 14;
    ctx.strokeStyle = gradient;
    ctx.lineWidth = strokeWidth * 2.2;
    ctx.lineJoin = 'round';
    ctx.lineCap = 'round';
    ctx.beginPath();
    points.forEach((point, index) => {
      if (index === 0) ctx.moveTo(point.topX, point.topY);
      else ctx.lineTo(point.topX, point.topY);
    });
    ctx.stroke();
    ctx.restore();
  }

  ctx.save();
  ctx.strokeStyle = 'rgba(255,255,255,0.55)';
  ctx.lineWidth = strokeWidth * 1.5;
  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  ctx.beginPath();
  points.forEach((point, index) => {
    if (index === 0) ctx.moveTo(point.topX, point.topY);
    else ctx.lineTo(point.topX, point.topY);
  });
  ctx.stroke();

  ctx.strokeStyle = gradient;
  ctx.lineWidth = strokeWidth;
  ctx.beginPath();
  points.forEach((point, index) => {
    if (index === 0) ctx.moveTo(point.topX, point.topY);
    else ctx.lineTo(point.topX, point.topY);
  });
  ctx.stroke();
  ctx.restore();
}

function drawEndpoints(
  ctx: CanvasRenderingContext2D,
  points: PreparedPoint[],
  gradientName: string,
  strokeWidth: number,
): void {
  if (points.length < 2) return;
  const start = points[0];
  const end = points[points.length - 1];
  const radius = strokeWidth * 2.2;
  ctx.save();
  ctx.fillStyle = interpolateGradient(gradientName as never, 0);
  ctx.beginPath();
  ctx.arc(start.topX, start.topY, radius, 0, Math.PI * 2);
  ctx.fill();

  ctx.globalAlpha = 0.95;
  ctx.fillStyle = interpolateGradient(gradientName as never, 1);
  ctx.beginPath();
  ctx.arc(end.topX, end.topY, radius, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
}

function drawStats(
  ctx: CanvasRenderingContext2D,
  stats: FrameRenderInput['stats'],
  options: FrameRenderInput['options'],
): void {
  if (!stats.length) return;
  const startX = options.padding + 14;
  const startY = options.padding + 28;
  const fontSize = clamp(options.height * 0.024, 12, 34);
  const lineGap = clamp(fontSize * 1.38, 18, 52);
  const labelDx = clamp(fontSize * 6.1, 72, 280);

  stats.forEach((entry, index) => {
    const y = startY + index * lineGap;
    const t = stats.length <= 1 ? 0.5 : index / (stats.length - 1);
    const color = interpolateGradient(options.gradient, t);
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.78;
    ctx.font = `600 ${Math.round(fontSize * 0.68)}px Geist Sans, sans-serif`;
    ctx.fillText(entry.label, startX, y);
    ctx.globalAlpha = 1;
    ctx.font = `700 ${Math.round(fontSize)}px Geist Sans, sans-serif`;
    ctx.fillText(entry.value, startX + labelDx, y);
  });
}

function remapContrast(value: number): number {
  const v = clamp(value, 0, 1);
  return clamp((v - 0.5) * 1.55 + 0.5, 0, 1);
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
