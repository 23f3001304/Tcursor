import type { ClickSample } from "../../lib/ipc";

/** Client-side mirror of the export's click-ripple rendering (joins the camZoomAction.ts/
 *  layoutAt.ts/spotlightPreview.ts TS-mirror family). Click ripples used to draw ONLY inside the
 *  backend FX-overlay PNG (`fxOverlay.ts`), requested at `FX_BUCKET_MS` cadence with single-flight
 *  gating - during playback that reads as laggy/stuck rings, the highest-frequency element on
 *  screen getting the coarsest update rate. Drawing them here instead, straight on the preview
 *  canvas every rAF tick, fixes that FOR `stylesMirrored`'s two styles.
 *
 *  `stylesMirrored` is ONLY {"ripple" (the default), "shockwave"} - the two styles this pass
 *  prioritizes. The other four (Pulse/Glow/Neon/Particles) are NOT drawn here - instead
 *  `overlayNeedsClicks` gates `fxOverlay.ts`'s request so THOSE styles keep going through the
 *  backend overlay exactly as before (laggy, but still visible - not a silent regression to
 *  nothing). A follow-up extending `stylesMirrored` is a matter of adding their curves below from
 *  the same Rust/GPU sources cited per-function, at which point `overlayNeedsClicks` needs no
 *  changes itself - it already excludes whatever's in `stylesMirrored`.
 *
 *  Every curve here is checked against its source line-for-line: `fx.wgsl` (the GPU shader the
 *  export/preview actually render with - `select_fx` prefers GPU, `fx_state.rs`) is the primary
 *  reference; `export/fx/clickdraw.rs` (the CPU fallback) is cited too where it independently
 *  documents the same simplifications this module makes (e.g. Shockwave's UV-warp). */

/** Click-effect lifetime, ms. MUST equal the export's `LIFE_MS` (`fx_state.rs`) and the OLD
 *  `fxOverlay.ts` `RIPPLE_MS` (superseded by this - progress is elapsed/lifetime on both sides). */
export const RIPPLE_LIFE_MS = 600;

/** The click-fx styles this module draws. Checked by both the caller (skip the work entirely for
 *  an unmirrored style) and `drawRipplePreview` itself (safe to call standalone/in tests without
 *  relying on caller discipline). */
export const stylesMirrored: ReadonlySet<string> = new Set(["ripple", "shockwave"]);

/** Whether `fxOverlay.ts`'s backend request still needs to carry click hits for `style`: true for
 *  every REAL click-fx style this module does NOT mirror (Pulse/Glow/Neon/Particles) - those keep
 *  falling back to the overlay exactly as before this pass (laggy, at FX_BUCKET_MS cadence, but
 *  still visible - better than disappearing from the live preview entirely). False for
 *  `stylesMirrored` (drawn client-side instead, see `drawMirroredRipples`) and for `"none"`
 *  (nothing to draw either way). */
export function overlayNeedsClicks(style: string): boolean {
  return style !== "none" && !stylesMirrored.has(style);
}

/** One active click: its raw 0..1 screen-content position (same basis `ClickSample` uses - NOT
 *  yet mapped to canvas px, that's the caller's `mapFn`'s job) and progress 0..1 through its life. */
export interface ActiveHit { x: number; y: number; progress: number }

/** One active click ALREADY mapped to canvas px, ready for `drawRipplePreview`. */
export interface PixelHit { x: number; y: number; progress: number }

const clamp01 = (v: number) => Math.min(Math.max(v, 0), 1);

/** TS mirror of `export/fx/clickfx.rs::hits_at` - which clicks are "alive" at `now` and how far
 *  through their life each one is. Rust: `et >= e.t && et - e.t < life_ms` (inclusive start,
 *  EXCLUSIVE end) - the old inline version in `fxOverlay.ts` used an inclusive end (`dt > life`),
 *  an off-by-one against the export only visible on the single frame exactly at the boundary;
 *  fixed here since this is now the one place that math lives. */
export function activeRippleHits(clicks: ClickSample[], now: number): ActiveHit[] {
  const out: ActiveHit[] = [];
  for (const c of clicks) {
    const dt = now - c.t;
    if (dt < 0 || dt >= RIPPLE_LIFE_MS) continue;
    out.push({ x: c.x, y: c.y, progress: dt / RIPPLE_LIFE_MS });
  }
  return out;
}

/** TS mirror of fx.wgsl's per-hit `a` (line 169) / `clickfx::fade_alpha` - fades 1->0 over the
 *  life, scaled by the user's intensity setting. */
export function rippleAlpha(progress: number, intensity: number): number {
  return clamp01(1 - clamp01(progress)) * clamp01(intensity);
}

/** TS mirror of fx.wgsl line 172 (`FX_RIPPLE` radius) / `clickfx::ripple_radius` - ring radius in
 *  OUTPUT px, growing linearly to `oh * 0.06` over the life. `oh` is the OUTPUT frame height in
 *  px (the export's `oh`; in the preview, the real canvas height - see `useCompositeLoop.ts`'s
 *  doc on why that's the right basis, not the downscaled FX-request resolution). */
export function rippleRadiusPx(progress: number, oh: number): number {
  return clamp01(progress) * oh * 0.06;
}
/** TS mirror of fx.wgsl line 173 (`FX_RIPPLE` thickness). */
export function rippleThicknessPx(oh: number): number {
  return Math.max(oh * 0.006, 1);
}

/** TS mirror of fx.wgsl line 186 (`FX_SHOCKWAVE` ring radius) - the ring-draw half only. The
 *  shader ALSO warps nearby background pixels radially around this ring (lines 96-108, a
 *  screen-space UV refraction) - a per-pixel background resample this 2D canvas draw can't
 *  reproduce cheaply. `clickdraw.rs`'s own CPU fallback makes the identical trade (see its
 *  `ClickFxStyle::Shockwave` comment: "the CPU path can't do" the warp either) and compensates
 *  with the same fainter `SHOCKWAVE_ALPHA_MUL` this mirrors - so the ring alone, without the
 *  warp, is an established simplification in this codebase, not a new one. */
export function shockwaveRadiusPx(progress: number, oh: number): number {
  return clamp01(progress) * oh * 0.09;
}
/** TS mirror of fx.wgsl line 187 (`FX_SHOCKWAVE` thickness). */
export function shockwaveThicknessPx(oh: number): number {
  return Math.max(oh * 0.008, 1);
}
/** fx.wgsl line 188: the shockwave ring is additive at 0.5x the base alpha - deliberately fainter
 *  than Neon's 1.4x since the shader leans on the warp above for the rest of its punch. */
export const SHOCKWAVE_ALPHA_MUL = 0.5;

/** Compute this tick's active, style-mirrored hits and draw them on the real canvas - the glue
 *  `useCompositeLoop.ts` calls straight from its rAF tick. `enabled` is `clickfx.enabled`, checked
 *  FIRST (mirrors `fxOverlay.ts`'s `requestFxOverlay`, whose very first check is the same field -
 *  the master switch for the whole FX stack, spotlight AND clicks; without it here, turning FX off
 *  still drew ripples in the live preview while the export correctly drew none). `mapFn` (built in
 *  the caller) outputs `fxW`x`fxH`-space px, the resolution the backend renders the spotlight/
 *  video-fx overlay at; scaling its result by `(canvasW/fxW, canvasH/fxH)` reproduces the IDENTICAL
 *  panel/zoom mapping at the real canvas's resolution instead - the mapping is linear/homogeneous
 *  in fxW/fxH (every intermediate quantity `mapFn` derives from - dx/dy/dw/dh/cx0/cy0/cw/ch -
 *  scales with it), so this is equivalent to recomputing that whole geometry a second time at full
 *  scale, without actually doing so. Radius/thickness then use the real canvas height directly for
 *  the same reason (see `rippleRadiusPx`'s doc). No-ops via `drawRipplePreview` for an unmirrored
 *  style or no active hits - the (cheap) hit-list/mapping work below still runs either way, guarded
 *  by `stylesMirrored` up front so it's skipped entirely for a style this module doesn't draw. */
export function drawMirroredRipples(
  ctx: CanvasRenderingContext2D, clicks: ClickSample[], now: number,
  enabled: boolean, style: string, color: [number, number, number], intensity: number,
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

/** Draw every active, style-mirrored hit onto `ctx` (already the real canvas, full resolution -
 *  see `useCompositeLoop.ts` for how hit positions get there). No-ops for a style not in
 *  `stylesMirrored` or an empty hit list. Ripple blends normally (mirrors fx.wgsl's `mix`, an
 *  over-composite); Shockwave is additive (`globalCompositeOperation = "lighter"`, mirrors the
 *  shader's `color = color + ...`). Each ring's soft triangular coverage falloff (`(thick -
 *  |d-radius|) / thick`, clamped 0..1 - the exact per-pixel shape fx.wgsl computes) is reproduced
 *  losslessly via a 3-stop radial gradient: canvas gradients interpolate LINEARLY between stops,
 *  which is exactly what that falloff already is (a linear ramp up to the ring, then back down) -
 *  no per-pixel loop needed on the 2D canvas. */
export function drawRipplePreview(
  ctx: CanvasRenderingContext2D, hits: PixelHit[], style: string,
  color: [number, number, number], intensity: number, oh: number,
): void {
  if (!stylesMirrored.has(style) || hits.length === 0 || oh <= 0) return;
  const [r, g, b] = color;
  const shockwave = style === "shockwave";
  ctx.save();
  ctx.globalCompositeOperation = shockwave ? "lighter" : "source-over";
  for (const h of hits) {
    const baseA = rippleAlpha(h.progress, intensity) * (shockwave ? SHOCKWAVE_ALPHA_MUL : 1);
    if (baseA <= 0) continue;
    const radius = shockwave ? shockwaveRadiusPx(h.progress, oh) : rippleRadiusPx(h.progress, oh);
    const thick = shockwave ? shockwaveThicknessPx(oh) : rippleThicknessPx(oh);
    const outer = radius + thick;
    if (outer <= 0) continue;
    // innerStop/peakStop mirror the shader's `abs(d - radius) <= thick` band exactly: at d=0 with
    // radius < thick (the ripple's very first ~10% of life), cov(0) is already > 0 - the tent's
    // peak offset stays clamped >= its base offset so that region degrades to a soft disc instead
    // of an inverted gradient, matching what the shader itself produces there.
    const innerStop = Math.max(0, (radius - thick) / outer);
    const peakStop = Math.min(1, Math.max(innerStop, radius / outer));
    const grad = ctx.createRadialGradient(h.x, h.y, 0, h.x, h.y, outer);
    grad.addColorStop(innerStop, `rgba(${r},${g},${b},0)`);
    grad.addColorStop(peakStop, `rgba(${r},${g},${b},${baseA})`);
    grad.addColorStop(1, `rgba(${r},${g},${b},0)`);
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(h.x, h.y, outer, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}
