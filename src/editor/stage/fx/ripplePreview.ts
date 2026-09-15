import type { ClickSample } from "../../../shared/ipc";

export const RIPPLE_LIFE_MS = 600;

export const stylesMirrored: ReadonlySet<string> = new Set(["ripple", "shockwave", "pulse"]);

export function overlayNeedsClicks(style: string): boolean {
  return style !== "none" && !stylesMirrored.has(style);
}

export interface ActiveHit {
  x: number;
  y: number;
  progress: number;
}

export interface PixelHit {
  x: number;
  y: number;
  progress: number;
}

type RGB = [number, number, number];
const clamp01 = (v: number) => Math.min(Math.max(v, 0), 1);
const rgba = (c: RGB, a: number) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
const WHITE: RGB = [255, 255, 255];

export function smoothstep(e0: number, e1: number, x: number): number {
  const t = clamp01((x - e0) / (e1 - e0));
  return t * t * (3 - 2 * t);
}

export function easeOut(progress: number): number {
  const q = 1 - clamp01(progress);
  return 1 - q * q * q;
}

export function rippleAlpha(progress: number, intensity: number): number {
  return (1 - smoothstep(0.55, 1, clamp01(progress))) * clamp01(intensity);
}

export function activeRippleHits(clicks: ClickSample[], now: number): ActiveHit[] {
  const out: ActiveHit[] = [];
  for (const c of clicks) {
    const dt = now - c.t;
    if (dt < 0 || dt >= RIPPLE_LIFE_MS) continue;
    out.push({ x: c.x, y: c.y, progress: dt / RIPPLE_LIFE_MS });
  }
  return out;
}

export function rippleRadiusPx(progress: number, oh: number): number {
  return easeOut(progress) * oh * 0.06;
}

export const RIPPLE_RINGS: ReadonlyArray<{ offset: number; thick: number; gain: number }> = [
  { offset: 0, thick: 0.006, gain: 1 },
  { offset: 0.15, thick: 0.0045, gain: 0.7 },
  { offset: 0.3, thick: 0.003, gain: 0.45 },
];
export function rippleThicknessPx(oh: number): number {
  return Math.max(oh * 0.006, 1);
}

export function shockwaveRadiusPx(progress: number, oh: number): number {
  return easeOut(progress) * oh * 0.09;
}
export function shockwaveThicknessPx(oh: number): number {
  return Math.max(oh * 0.008, 1);
}

export const SHOCKWAVE_ALPHA_MUL = 0.5;

export function pulseRadiusPx(progress: number, oh: number): number {
  return oh * (0.01 + 0.025 * easeOut(progress));
}

function fill(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  r: number,
  g: CanvasGradient | string,
): void {
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fill();
}

function ringBand(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  radius: number,
  thick: number,
  c: RGB,
  a: number,
): void {
  const outer = radius + thick;
  if (outer <= 0 || a <= 0) return;
  const inner = Math.max(0, (radius - thick) / outer);
  const peak = Math.min(1, Math.max(inner, radius / outer));
  const g = ctx.createRadialGradient(x, y, 0, x, y, outer);
  g.addColorStop(inner, rgba(c, 0));
  g.addColorStop(peak, rgba(c, a));
  g.addColorStop(1, rgba(c, 0));
  fill(ctx, x, y, outer, g);
}

function bloom(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, c: RGB, a: number): void {
  if (r <= 0 || a <= 0) return;
  const g = ctx.createRadialGradient(x, y, 0, x, y, r * 2);
  for (const s of [0, 0.25, 0.5, 0.75, 1]) g.addColorStop(s, rgba(c, a * Math.exp(-((s * 2) ** 2))));
  fill(ctx, x, y, r * 2, g);
}

function softDisc(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, c: RGB, a: number): void {
  if (r <= 0 || a <= 0) return;
  const g = ctx.createRadialGradient(x, y, 0, x, y, r);
  for (const s of [0, 0.2, 0.4, 0.6, 0.8, 1]) g.addColorStop(s, rgba(c, a * (1 - smoothstep(0.2, 1, s))));
  fill(ctx, x, y, r, g);
}

export function drawMirroredRipples(
  ctx: CanvasRenderingContext2D,
  clicks: ClickSample[],
  now: number,
  enabled: boolean,
  style: string,
  color: RGB,
  intensity: number,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  fxW: number,
  fxH: number,
  canvasW: number,
  canvasH: number,
): void {
  if (!enabled || !stylesMirrored.has(style) || clicks.length === 0) return;
  const upX = canvasW / fxW,
    upY = canvasH / fxH;
  const hits: PixelHit[] = [];
  for (const h of activeRippleHits(clicks, now)) {
    const p = mapFn(h.x, h.y);
    if (p) hits.push({ x: p[0] * upX, y: p[1] * upY, progress: h.progress });
  }
  drawRipplePreview(ctx, hits, style, color, intensity, canvasH);
}

export function drawRipplePreview(
  ctx: CanvasRenderingContext2D,
  hits: PixelHit[],
  style: string,
  color: RGB,
  intensity: number,
  oh: number,
): void {
  if (!stylesMirrored.has(style) || hits.length === 0 || oh <= 0) return;
  const inten = clamp01(intensity);
  ctx.save();
  for (const h of hits) {
    const p = clamp01(h.progress);
    const a = rippleAlpha(p, intensity);
    ctx.globalCompositeOperation = "lighter";
    bloom(ctx, h.x, h.y, oh * 0.015, WHITE, (1 - smoothstep(0, 0.14, p)) * inten);
    if (a <= 0) continue;
    if (style === "ripple") {
      bloom(ctx, h.x, h.y, rippleRadiusPx(p, oh) * 2, color, a * 0.15);
      ctx.globalCompositeOperation = "source-over";
      for (const r of RIPPLE_RINGS) {
        if (p <= r.offset && r.offset > 0) continue;
        ringBand(
          ctx,
          h.x,
          h.y,
          rippleRadiusPx(p - r.offset, oh),
          Math.max(oh * r.thick, 1),
          color,
          a * r.gain,
        );
      }
    } else if (style === "shockwave") {
      const r = shockwaveRadiusPx(p, oh);
      ringBand(ctx, h.x, h.y, r, shockwaveThicknessPx(oh), color, a * SHOCKWAVE_ALPHA_MUL);
      ringBand(ctx, h.x, h.y, r + oh * 0.004, Math.max(oh * 0.004, 1), WHITE, a * 0.35);
    } else {
      const r = pulseRadiusPx(p, oh);
      ctx.globalCompositeOperation = "source-over";
      softDisc(ctx, h.x, h.y, r, color, a);
      ctx.globalCompositeOperation = "lighter";
      ringBand(ctx, h.x, h.y, r, Math.max(oh * 0.004, 1), color, a * 0.6);
      fill(ctx, h.x, h.y, oh * 0.006, rgba(WHITE, a * (1 - smoothstep(0, 0.3, p))));
    }
  }
  ctx.restore();
}
