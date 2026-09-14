# src/editor/stage/ripplePreview.ts

Client-side mirror of the export's click FX (joins the `camZoomAction.ts`/`layoutAt.ts`/`spotlightPreview.ts` TS-mirror family). Click effects used to draw ONLY inside the backend FX-overlay PNG (`fxOverlay.ts`), requested at `FX_BUCKET_MS` cadence with single-flight gating - during playback that read as laggy/stuck rings, the highest-frequency element on screen getting the coarsest update rate. Drawing them here instead, straight on the preview canvas every rAF tick (`useCompositeLoop.ts`), fixes that for `stylesMirrored`'s styles.

**Fallback, not a gap.** `stylesMirrored` is `{"ripple" (the default), "shockwave", "pulse"}`. The other three click-fx styles (Glow/Neon/Particles) are NOT drawn here - `overlayNeedsClicks` instead gates `fxOverlay.ts`'s request so THOSE keep going through the backend overlay (laggy at `FX_BUCKET_MS` cadence, but still visible - not a silent regression to nothing). Extending the set is a matter of adding their curves below from the same sources cited per-function here; `overlayNeedsClicks` needs no changes itself, since it already excludes whatever is in `stylesMirrored`.

Every curve is checked against its source line-for-line: `export/fx/fx_clicks.wgsl` (the GPU shader the export and the preview actually render with - `select_fx` prefers GPU - and the reference look) is the primary reference; `export/fx/clickdraw.rs` (the CPU fallback) is cited where it independently documents the same simplification this module makes (Shockwave's refraction).

**What this module deliberately does not reproduce.** Shockwave's band in the shader also warps the sampled background radially and splits R and B 2 px either side of that displacement (`fx.wgsl`, before the texture sample). That is a per-pixel resample of the frame underneath, which a 2D canvas draw cannot do cheaply; `clickdraw.rs` makes the identical trade for the identical reason. Here Shockwave is its ring plus its white leading rim. Everything else - the shared timing, the impact flash, Ripple's three rings and halo, Pulse's disc, rim and core - is mirrored exactly.

## RIPPLE_LIFE_MS

```ts
export const RIPPLE_LIFE_MS = 600;
```

Click-effect lifetime, ms. MUST equal the export's `LIFE_MS` (`fx_state.rs`) - progress is elapsed/lifetime on both sides.

## stylesMirrored

```ts
export const stylesMirrored: ReadonlySet<string>;
```

The click-fx styles this module draws (`"ripple"`, `"shockwave"`, `"pulse"`). Checked by both the caller (`drawMirroredRipples`, skip the work entirely for an unmirrored style) and `drawRipplePreview` itself (safe to call standalone/in tests without relying on caller discipline).

### Behaviors

- covers `"ripple"`, `"shockwave"` and `"pulse"`; does not cover `"glow"`, `"particles"`, `"neon"` or `"none"`.

## overlayNeedsClicks

```ts
export function overlayNeedsClicks(style: string): boolean
```

Whether `fxOverlay.ts`'s backend request still needs to carry click hits for `style`: `true` for every REAL click-fx style this module does NOT mirror (Glow/Neon/Particles), `false` for `stylesMirrored` (drawn client-side instead) and for `"none"` (nothing to draw either way): `style !== "none" && !stylesMirrored.has(style)`.

### Behaviors

- `false` for all three mirrored styles; `true` for each of the three unmirrored ones; `false` for `"none"`.

### Used by

- `fxOverlay.ts` (`requestFxOverlay`) - gates whether it builds a `hits` array at all for the backend request.
- `useCompositeLoop.ts` / `fxRequestTick.ts` - gate whether the FX-request cache key varies with clicks (it does not when this is `false`).

## ActiveHit

```ts
export interface ActiveHit { x: number; y: number; progress: number }
```

`x`/`y` are a 0..1 CANVAS position - a fraction of the recorded frame, NOT of the screen panel, and not yet in px. Mapping them is the caller's `mapFn`'s job, and after a mid-take display switch the two spaces differ: pass `fxFrameGeometry`'s `mapCanvas`, which folds in the active source span's crop the way Rust's `to_panel` does.

`activeRippleHits`' output: one active click's raw 0..1 screen-content position (the basis `ClickSample` uses, NOT yet mapped to canvas px) and its progress 0..1 through its life. Structurally identical to `PixelHit`, kept as a distinct name for the coordinate space (pre-mapping).

## PixelHit

```ts
export interface PixelHit { x: number; y: number; progress: number }
```

What `drawRipplePreview` actually draws: one active click already mapped to canvas px by the caller. Structurally identical to `ActiveHit`, kept as a distinct name for the coordinate space (post-mapping).

## smoothstep

```ts
export function smoothstep(e0: number, e1: number, x: number): number
```

The GPU builtin, `t * t * (3 - 2t)` over the clamped `(x - e0) / (e1 - e0)`. Every feather and fade below is written in terms of it so the curves are the shader's, not an approximation of them.

### Behaviors

- 0 below `e0`, 1 above `e1`, 0.5 at the midpoint.

## easeOut

```ts
export function easeOut(progress: number): number
```

TS mirror of `fx_clicks.wgsl::fx_ease` / `clickfx.rs::ease_out` - the shared radius easing for every click style, ease-out cubic `1 - (1 - p)^3`. Clamps outside 0..1.

### Behaviors

- pinned at the SAME five points as the Rust test `ease_out_is_pinned_at_five_points`: 0 / 0.578125 / 0.875 / 0.984375 / 1; clamps at -1 and 2.

## rippleAlpha

```ts
export function rippleAlpha(progress: number, intensity: number): number
```

TS mirror of `fx_clicks.wgsl::fx_alpha` / `clickfx.rs::fade_alpha` - the full intensity for the first 55% of the life, then a smoothstep release to exactly 0: `(1 - smoothstep(0.55, 1, p)) * intensity`. Keeps its old name because every caller and the whole test file already use it.

### Behaviors

- pinned at the SAME five points as the Rust test `fade_alpha_is_pinned_at_five_points`: 1 / 1 / 1 / 0.5829904 / 0; `rippleAlpha(0.5, 0.5) === 0.5` (inside the hold, matching the Rust `fade_alpha(0.5, 0.5)`); clamps progress and intensity outside 0..1.

## activeRippleHits

```ts
export function activeRippleHits(clicks: ClickSample[], now: number): ActiveHit[]
```

TS mirror of `export/fx/clickfx.rs::hits_at` - which clicks are "alive" at `now` and how far through their life each one is. Rust: `et >= e.t && et - e.t < life_ms` (inclusive start, EXCLUSIVE end) - the old inline version in `fxOverlay.ts` used an inclusive end (`dt > life`), an off-by-one against the export only visible on the single frame exactly at the boundary; fixed here since this is now the one place that math lives.

### Behaviors

- includes a click exactly at its start, excludes one exactly at its life boundary, computes `progress` as elapsed/life, excludes a future click, keeps every simultaneously-alive click in order.

## rippleRadiusPx

```ts
export function rippleRadiusPx(progress: number, oh: number): number
```

Ripple's lead-ring radius in OUTPUT px: `easeOut(progress) * oh * 0.06`. `oh` is the OUTPUT frame height (the export's `oh`; in the preview, the real canvas height - see `drawMirroredRipples`). The 2nd and 3rd rings are this same function at `progress - 0.15` / `progress - 0.30`.

### Behaviors

- 0 at progress 0, exactly `oh * 0.06` at progress 1, **52.5px at progress 0.5 for `oh = 1000`** (eased, pinning that it is no longer the linear 30), clamped past 1.

## RIPPLE_RINGS

```ts
export const RIPPLE_RINGS: ReadonlyArray<{ offset: number; thick: number; gain: number }>;
```

Ripple's three rings, mirroring `fx_clicks.wgsl`'s `FX_RIPPLE` branch: launch offsets 0 / 0.15 / 0.30 of the life (0, 90 and 180 ms), thicknesses 0.6% / 0.45% / 0.3% of height, alpha gains 1 / 0.7 / 0.45. A ring only exists once its offset has passed.

*Why a table rather than three inline draws:* the three differ only in these numbers, and having them in one place is what makes "each thinner and fainter than the last" checkable in a test instead of by reading.

### Behaviors

- the three offsets, thicknesses and gains are pinned exactly; the offsets times `RIPPLE_LIFE_MS` are the 90 ms and 180 ms launch delays.

## rippleThicknessPx

```ts
export function rippleThicknessPx(oh: number): number
```

The lead ring's thickness, `max(oh * 0.006, 1)` - the shader's floor included, so a tiny preview canvas still draws a 1px line instead of nothing.

### Behaviors

- 6px at `oh = 1000`; floors at 1px for `oh = 100`.

## shockwaveRadiusPx

```ts
export function shockwaveRadiusPx(progress: number, oh: number): number
```

Shockwave's ring radius, `easeOut(progress) * oh * 0.09` - the ring-draw half only (see the refraction note at the top).

### Behaviors

- 0 at progress 0, `oh * 0.09` at progress 1, 78.75px at progress 0.5 for `oh = 1000` (eased, not the linear 45).

## shockwaveThicknessPx

```ts
export function shockwaveThicknessPx(oh: number): number
```

`max(oh * 0.008, 1)`.

### Behaviors

- 8px at `oh = 1000`; floors at 1px for `oh = 50`.

## SHOCKWAVE_ALPHA_MUL

```ts
export const SHOCKWAVE_ALPHA_MUL = 0.5;
```

The shockwave ring is additive at 0.5x the base alpha - deliberately fainter than Neon's 1.4x, since the shader leans on the refraction for the rest of its punch.

### Behaviors

- is exactly 0.5.

## pulseRadiusPx

```ts
export function pulseRadiusPx(progress: number, oh: number): number
```

Pulse's disc radius, `oh * (0.01 + 0.025 * easeOut(progress))` - 1% of height growing to 3.5%.

### Behaviors

- 10px then 35px at `oh = 1000` for progress 0 and 1; 31.875px at progress 0.5 (eased).

## drawMirroredRipples

```ts
export function drawMirroredRipples(
  ctx, clicks, now, enabled, style, color, intensity, mapFn, fxW, fxH, canvasW, canvasH): void
```

Computes this tick's active, style-mirrored hits and draws them on the real canvas - the glue `useCompositeLoop.ts` calls straight from its rAF tick. No-ops (without touching `ctx`) for disabled FX, an unmirrored style, or no clicks.

- `enabled` is `clickfx.enabled`, checked FIRST. It is the master switch for the whole FX stack, spotlight included - the export returns before drawing anything when it is off (`fx_state.rs`'s `render` gate), and `fxOverlay.ts`'s `requestFxOverlay` treats it as its own first check for the same reason. Without it here, turning "Click animations" off still drew effects in the live preview while the export correctly drew none.
- `mapFn` (built in the caller) outputs `fxW`x`fxH`-space px, the resolution the backend renders the spotlight/video-fx overlay at. Scaling its result by `(canvasW/fxW, canvasH/fxH)` reproduces the IDENTICAL panel/zoom mapping at the real canvas's resolution, because the mapping is linear/homogeneous in `fxW`/`fxH` (every intermediate quantity `mapFn` derives from scales with it) - equivalent to recomputing that whole geometry at full scale without doing so. Radii then use the real canvas height directly for the same reason.

### Behaviors

- never touches `ctx` when `enabled` is false (the regression above), when the style is outside `stylesMirrored`, or when the click list is empty.

## drawRipplePreview

```ts
export function drawRipplePreview(ctx, hits: PixelHit[], style, color, intensity, oh: number): void
```

Draws every active, style-mirrored hit onto `ctx` (already the real canvas, full resolution). No-ops for a style not in `stylesMirrored`, an empty hit list, or a non-positive `oh`.

### What each style draws

Every style opens with the shared **impact flash**: an additive white gaussian bloom of radius `oh * 0.015` scaled by `(1 - smoothstep(0, 0.14, p)) * intensity`, so the click reads as landing at a point about 80 ms before the style's own geometry has grown into anything.

- **ripple** - an additive tint halo (gaussian at twice the lead ring's radius, gain `a * 0.15`), then the three `RIPPLE_RINGS` blended normally (the shader's `mix`, an over-composite).
- **shockwave** - the tint ring additive at `SHOCKWAVE_ALPHA_MUL`, plus a white leading rim `oh * 0.004` outside it at gain `a * 0.35`.
- **pulse** - the feathered disc blended normally, then an additive tint rim on its edge at gain `a * 0.6` and a solid additive white core of radius `oh * 0.006` faded out by `p = 0.3`.

### How the shapes are drawn on a 2D canvas

- **Rings** (`ringBand`): each ring's triangular coverage falloff (`(thick - |d - radius|) / thick`, clamped 0..1 - the exact per-pixel shape the shader computes) is reproduced losslessly by a 3-stop radial gradient, because canvas gradients interpolate LINEARLY between stops and that falloff already is a linear ramp up to the ring then back down. No per-pixel loop needed. The peak stop stays clamped at or above the base stop so a radius smaller than `thick` (a ring's first frames) degrades to a soft disc rather than an inverted gradient - which is what the shader itself produces there.
- **Blooms** (`bloom`): `exp(-(d/r)^2)` out to `2r`, sampled at five stops (1, 0.7788, 0.3679, 0.1054, 0.0183). An approximation, but of a curve that is itself a soft glow, so the five stops are visually indistinguishable from the shader's per-pixel gaussian.
- **Pulse's body** (`softDisc`): `1 - smoothstep(0.2, 1, d/r)` sampled at six stops. *Why the feather starts at a fifth of the radius:* a disc solid most of the way out reads as a flat sticker, which is what the old fixed-radius Pulse looked like; `clickdraw.rs`'s `soft_disc` uses the identical `0.2`.
- **Composite modes**: `"lighter"` for everything the shader writes as `color + ...` (the flash, the halo, every rim and core, Shockwave's rings), `"source-over"` for what it writes as `mix(...)` (Ripple's rings, Pulse's body).

### Behaviors

- never touches `ctx` for an unmirrored style, an empty hit list, or `oh <= 0`.
