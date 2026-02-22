import type { GradientName } from '../types/api';

const gradientMap: Record<GradientName, string[]> = {
  fire: ['#FF3366', '#FF6600', '#FF9933'],
  ocean: ['#0055FF', '#0099DD', '#00D1FF'],
  sunset: ['#FF2D55', '#FF7E5F', '#FEB47B'],
  forest: ['#1D976C', '#4CD964', '#93F9B9'],
  violet: ['#FF0080', '#8E2DE2', '#4A00E0'],
  rideviz: ['#00C2FF', '#00EABD', '#00FF94'],
  white: ['#FFFFFF', '#FFFFFF', '#FFFFFF'],
  black: ['#000000', '#000000', '#000000'],
};

export function getGradientStops(name: GradientName): string[] {
  return gradientMap[name] ?? gradientMap.fire;
}

export function interpolateGradient(name: GradientName, t: number): string {
  const stops = getGradientStops(name);
  const clamped = Math.max(0, Math.min(1, t));
  if (stops.length <= 1) return stops[0] ?? '#FFFFFF';

  const scaled = clamped * (stops.length - 1);
  const index = Math.min(stops.length - 2, Math.floor(scaled));
  const localT = scaled - index;

  const start = hexToRgb(stops[index]);
  const end = hexToRgb(stops[index + 1]);
  const r = Math.round(start.r + (end.r - start.r) * localT);
  const g = Math.round(start.g + (end.g - start.g) * localT);
  const b = Math.round(start.b + (end.b - start.b) * localT);
  return `rgb(${r}, ${g}, ${b})`;
}

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const clean = hex.replace('#', '');
  if (clean.length !== 6) return { r: 255, g: 255, b: 255 };
  return {
    r: parseInt(clean.slice(0, 2), 16),
    g: parseInt(clean.slice(2, 4), 16),
    b: parseInt(clean.slice(4, 6), 16),
  };
}
