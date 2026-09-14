import type { ClickSample } from "../../lib/ipc";

/** Client-side mirror of the export's click FX (joins the camZoomAction.ts/layoutAt.ts/
 *  spotlightPreview.ts TS-mirror family). Click effects used to draw ONLY inside the backend
 *  FX-overlay PNG (`fxOverlay.ts`), requested at `FX_BUCKET_MS` cadence with single-flight
 *  gating - during playback that reads as laggy/stuck rings, the highest-frequency element on
 *  screen getting the coarsest update rate. Drawing them here instead, straight on the preview
 *  canvas every rAF tick, fixes that for `stylesMirrored`'s styles; the other three
 *  (Glow/Neon/Particles) keep falling back to that overlay via `overlayNeedsClicks`. Every curve is
 *  checked against `export/fx/fx_clicks.wgsl` - the GPU shader the export and the preview actually
 *  render with (`select_fx` prefers GPU), and the reference look; see
 *  `docs/api/src/editor/stage/ripplePreview.md` for what is deliberately not reproduced. */

/** Click-effect lifetime, ms. MUST equal the export's `LIFE_MS` (`fx_state.rs`). */
export const RIPPLE_LIFE_MS = 600;

/** The click-fx styles this module draws. Checked by the caller AND by `drawRipplePreview`. */
export const stylesMirrored: ReadonlySet<string> = new Set(["ripple", "shockwave", "pulse"]);

/** Whether `fxOverlay.ts`'s backend request still needs to carry click hits for `style`: true for
 *  every REAL click-fx style this module does NOT mirror (Glow/Neon/Particles), false for
 *  `stylesMirrored` (drawn client-side) and for `"none"` (nothing to draw either way). */
export function overlayNeedsClicks(style: string): boolean {
  return style !== "none" && !stylesMirrored.has(style);
}

/** One active click: 0..1 CANVAS position (of the recorded frame, NOT the panel; `mapFn` puts it in px) + life progress. */
export interface ActiveHit { x: number; y: number; progress: number }
/** One active click ALREADY mapped to canvas px, ready for `drawRipplePreview`. */
export interface PixelHit { x: number; y: number; progress: number }

type RGB = [number, number, number];
const clamp01 = (v: number) => Math.min(Math.max(v, 0), 1);
const rgba = (c: RGB, a: number) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
const WHITE: RGB = [255, 255, 255];

/** The GPU builtin, for the curves below. */
export function smoothstep(e0: number, e1: number, x: number): number {
  const t = clamp01((x - e0) / (e1 - e0));
  return t * t * (3 - 2 * t);
}

/** TS mirror of `fx_clicks.wgsl::fx_ease` / `clickfx.rs::ease_out` - the shared radius easing,
 *  ease-out cubic. Pinned at the same five points as the Rust test. */
export function easeOut(progress: number): number {
  const q = 1 - clamp01(progress);
  return 1 - q * q * q;
}

/** TS mirror of `fx_clicks.wgsl::fx_alpha` / `clickfx.rs::fade_alpha` - full for the first 55% of
 *  the life, then a smoothstep release. Pinned at the same five points as the Rust test. */
export function rippleAlpha(progress: number, intensity: number): number {
  return (1 - smoothstep(0.55, 1, clamp01(progress))) * clamp01(intensity);
}

/** TS mirror of `export/fx/clickfx.rs::hits_at` - which clicks are alive at `now` and how far
 *  through their life each is. Rust: `et >= e.t && et - e.t < life_ms` (EXCLUSIVE end). */
export function activeRippleHits(clicks: ClickSample[], now: number): ActiveHit[] {
  const out: ActiveHit[] = [];
  for (const c of clicks) {
    const dt = now - c.t;
    if (dt < 0 || dt >= RIPPLE_LIFE_MS) continue;
    out.push({ x: c.x, y: c.y, progress: dt / RIPPLE_LIFE_MS });
  }
  return out;
}

/** Ripple's lead-ring radius in OUTPUT px (`oh` is the real canvas height - see
 *  `drawMirroredRipples`). The 2nd and 3rd rings are this curve at `progress - 0.15` / `- 0.30`. */
export function rippleRadiusPx(progress: number, oh: number): number {
  return easeOut(progress) * oh * 0.06;
}
/** Ripple's three rings: launch offsets 0 / 0.15 / 0.30 of the life (0, 90 and 180 ms),
 *  thicknesses 0.6% / 0.45% / 0.3% of height, alpha gains 1 / 0.7 / 0.45. */
export const RIPPLE_RINGS: ReadonlyArray<{ offset: number; thick: number; gain: number }> = [
  { offset: 0, thick: 0.006, gain: 1 },
  { offset: 0.15, thick: 0.0045, gain: 0.7 },
  { offset: 0.30, thick: 0.003, gain: 0.45 },
];
export function rippleThicknessPx(oh: number): number { return Math.max(oh * 0.006, 1); }

/** Shockwave's ring radius. The shader ALSO warps and chromatically splits the background across
 *  this band (fx.wgsl) - a per-pixel resample this canvas draw cannot do, nor can `clickdraw.rs`. */
export function shockwaveRadiusPx(progress: number, oh: number): number {
  return easeOut(progress) * oh * 0.09;
}
export function shockwaveThicknessPx(oh: number): number { return Math.max(oh * 0.008, 1); }
/** The shockwave ring is additive at 0.5x the base alpha - fainter than Neon's 1.4x because the
 *  shader leans on the refraction for the rest of its punch. */
export const SHOCKWAVE_ALPHA_MUL = 0.5;

/** Pulse's disc radius: 1% -> 3.5% of height, eased. */
export function pulseRadiusPx(progress: number, oh: number): number {
  return oh * (0.01 + 0.025 * easeOut(progress));
}

/** Fill a disc of radius `r` with a gradient or a flat colour, under the ctx's composite mode. */
function fill(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, g: CanvasGradient | string): void {
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.fill();
}

/** The shader's triangular ring coverage `(thick - |d - radius|) / thick` as a 3-stop radial
 *  gradient: canvas interpolates LINEARLY between stops, which is exactly that ramp. The peak stop
 *  stays clamped at or above the base one so a radius below `thick` degrades to a soft disc
 *  instead of an inverted gradient - what the shader produces there too. */
function ringBand(ctx: CanvasRenderingContext2D, x: number, y: number,
                  radius: number, thick: number, c: RGB, a: number): void {
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

/** `ck_gauss`: `exp(-(d/r)^2)` out to 2r, sampled at five stops. */
function bloom(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, c: RGB, a: number): void {
  if (r <= 0 || a <= 0) return;
  const g = ctx.createRadialGradient(x, y, 0, x, y, r * 2);
  for (const s of [0, 0.25, 0.5, 0.75, 1]) g.addColorStop(s, rgba(c, a * Math.exp(-((s * 2) ** 2))));
  fill(ctx, x, y, r * 2, g);
}

/** Pulse's body: solid to `r * 0.2`, smoothstepping away to nothing at `r`. */
function softDisc(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, c: RGB, a: number): void {
  if (r <= 0 || a <= 0) return;
  const g = ctx.createRadialGradient(x, y, 0, x, y, r);
  for (const s of [0, 0.2, 0.4, 0.6, 0.8, 1]) g.addColorStop(s, rgba(c, a * (1 - smoothstep(0.2, 1, s))));
  fill(ctx, x, y, r, g);
}

/** Compute this tick's active, style-mirrored hits and draw them on the real canvas - the glue
 *  `useCompositeLoop.ts` calls straight from its rAF tick. `enabled` is `clickfx.enabled`,
 *  checked FIRST (the master switch for the whole FX stack; without it here, turning FX off still
 *  drew effects in the live preview while the export correctly drew none). `mapFn` takes a CANVAS
 *  fraction and outputs `fxW`x`fxH` px - pass `fxFrameGeometry`'s `mapCanvas`, which folds in the
 *  active source span's crop the way Rust's `to_panel` does. Scaling by `(canvasW/fxW, canvasH/fxH)`
 *  reproduces that IDENTICAL mapping at the canvas's own resolution, being linear in fxW/fxH. */
export function drawMirroredRipples(
  ctx: CanvasRenderingContext2D, clicks: ClickSample[], now: number,
  enabled: boolean, style: string, color: RGB, intensity: number,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  fxW: number, fxH: number, canvasW: number, canvasH: number,
): void {
  if (!enabled || !stylesMirrored.has(style) || clicks.length === 0) return;
  const upX = canvasW / fxW, upY = canvasH / fxH;
  const hits: PixelHit[] = [];
  for (const h of activeRippleHits(clicks, now)) {
    const p = mapFn(h.x, h.y);
    if (p) hits.push({ x: p[0] * upX, y: p[1] * upY, progress: h.progress });
  }
  drawRipplePreview(ctx, hits, style, color, intensity, canvasH);
}

/** Draw every active, style-mirrored hit onto `ctx` (already the real canvas, full resolution).
 *  No-ops for a style not in `stylesMirrored` or an empty hit list. Every style opens with the
 *  shared white impact flash; `"lighter"` mirrors the shader's `color + ...`, `"source-over"` its
 *  `mix(...)` (Ripple's rings and Pulse's body). */
export function drawRipplePreview(
  ctx: CanvasRenderingContext2D, hits: PixelHit[], style: string,
  color: RGB, intensity: number, oh: number,
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
        ringBand(ctx, h.x, h.y, rippleRadiusPx(p - r.offset, oh), Math.max(oh * r.thick, 1), color, a * r.gain);
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
      // The white core is a SOLID disc in the shader (`clamp(oh*0.006 - d, 0, 1)`), not a bloom.
      fill(ctx, h.x, h.y, oh * 0.006, rgba(WHITE, a * (1 - smoothstep(0, 0.3, p))));
    }
  }
  ctx.restore();
}
