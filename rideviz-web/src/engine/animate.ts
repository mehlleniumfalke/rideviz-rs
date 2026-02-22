export function easeInOutSine(t: number): number {
  const clamped = Math.max(0, Math.min(1, t));
  return 0.5 * (1 - Math.cos(Math.PI * clamped));
}

export function buildAnimationProgress(frameIndex: number, frameCount: number): number {
  if (frameCount <= 1) return 1;
  const linear = frameIndex / (frameCount - 1);
  return easeInOutSine(linear);
}
