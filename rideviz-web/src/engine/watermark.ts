const DEFAULT_WATERMARK_TEXT = 'rideviz.online';

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function roundedRectPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
): void {
  const r = clamp(radius, 0, Math.min(width / 2, height / 2));
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + width, y, x + width, y + height, r);
  ctx.arcTo(x + width, y + height, x, y + height, r);
  ctx.arcTo(x, y + height, x, y, r);
  ctx.arcTo(x, y, x + width, y, r);
  ctx.closePath();
}

export function drawRideVizWatermark(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  text = DEFAULT_WATERMARK_TEXT,
): void {
  const fontSize = Math.max(13, Math.round(height * 0.02));
  const paddingX = Math.round(fontSize * 0.7);
  const paddingY = Math.round(fontSize * 0.5);
  const marginBottom = Math.max(16, Math.round(fontSize * 1.15));
  const x = width / 2;
  const y = height - marginBottom;

  ctx.save();

  ctx.textAlign = 'center';
  ctx.textBaseline = 'bottom';
  ctx.font = `${fontSize}px Geist Pixel, monospace`;

  const textWidth = ctx.measureText(text).width;
  const maxBoxWidth = Math.max(1, width - 12);
  const maxBoxHeight = Math.max(1, height - 12);
  const boxWidth = Math.min(maxBoxWidth, Math.ceil(textWidth + paddingX * 2));
  const boxHeight = Math.min(maxBoxHeight, Math.ceil(fontSize + paddingY * 2));
  const boxX = Math.round(x - boxWidth / 2);
  const boxY = Math.round(y - fontSize - paddingY);
  const radius = Math.round(Math.max(3, Math.min(6, fontSize * 0.3)));

  roundedRectPath(ctx, boxX, boxY, boxWidth, boxHeight, radius);
  ctx.shadowColor = 'rgba(0,0,0,0.12)';
  ctx.shadowBlur = 0;
  ctx.shadowOffsetX = 0;
  ctx.shadowOffsetY = 1;
  ctx.fillStyle = 'rgba(255,255,255,0.90)';
  ctx.fill();
  ctx.shadowColor = 'transparent';
  ctx.strokeStyle = 'rgba(0,0,0,0.92)';
  ctx.lineWidth = Math.max(1, Math.round(fontSize * 0.08));
  ctx.stroke();

  ctx.fillStyle = 'rgba(0,0,0,0.92)';
  ctx.fillText(text, x, y);

  ctx.restore();
}
