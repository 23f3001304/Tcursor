# src/editor/stage/ripplePreview.ts

Client-side mirror of the export's click-ripple rendering (joins the `camZoomAction.ts`/`layoutAt.ts`/`spotlightPreview.ts` TS-mirror family). Click ripples used to draw ONLY inside the backend FX-overlay PNG (`fxOverlay.ts`), requested at `FX_BUCKET_MS` cadence with single-flight gating - during playback that read as laggy/stuck rings, the highest-frequency element on screen getting the coarsest update rate. Drawing them here instead, straight on the preview canvas every rAF tick (`useCompositeLoop.ts`), fixes that for `stylesMirrored`'s two styles.

**Fallback, not a gap.** `stylesMirrored` is ONLY `{"ripple" (the default), "shockwave"}` - the two styles this pass prioritizes. The other four click-fx styles (Pulse/Glow/Neon/Particles) are NOT drawn here - `overlayNeedsClicks` instead gates `fxOverlay.ts`'s request so THOSE styles keep going through the backend overlay exactly as before this pass (laggy at `FX_BUCKET_MS` cadence, but still visible - not a silent regression to nothing). A follow-up extending `stylesMirrored` is a matter of adding their curves below from the same Rust/GPU sources cited per-function here; `overlayNeedsClicks` needs no changes itself when that happens, since it already excludes whatever's in `stylesMirrored`.

Every curve is checked against its source line-for-line: `fx.wgsl` (the GPU shader the export/preview actually render with - `select_fx` prefers GPU, `fx_state.rs`) is the primary reference; `export/fx/clickdraw.rs` (the CPU fallback) is cited too where it independently documents the same simplifications this module makes (e.g. Shockwave's UV-warp).

## RIPPLE_LIFE_MS

```ts
export const RIPPLE_LIFE_MS = 600;
```

Click-effect lifetime, ms. MUST equal the export's `LIFE_MS` (`fx_state.rs`) and the OLD `fxOverlay.ts` `RIPPLE_MS` constant it supersedes - progress is elapsed/lifetime on both sides.

## stylesMirrored

```ts
export const stylesMirrored: ReadonlySet<string>;
```

The click-fx styles this module draws (`"ripple"`, `"shockwave"`). Checked by both the caller (`drawMirroredRipples`, skip the work entirely for an unmirrored style) and `drawRipplePreview` itself (safe to call standalone/in tests without relying on caller discipline).

## overlayNeedsClicks

```ts
export function overlayNeedsClicks(style: string): boolean
```

Whether `fxOverlay.ts`'s backend request still needs to carry click hits for `style`: `true` for every REAL click-fx style this module does NOT mirror (Pulse/Glow/Neon/Particles) - those keep falling back to the overlay exactly as before this pass. `false` for `stylesMirrored` (drawn client-side instead) and for `"none"` (nothing to draw either way): `style !== "none" && !stylesMirrored.has(style)`.

### Behaviors

- `false` for both mirrored styles (`"ripple"`, `"shockwave"`); `true` for each of the four unmirrored styles; `false` for `"none"`.

### Used by

- `fxOverlay.ts` (`requestFxOverlay`) - gates whether it builds a `hits` array at all for the backend request.
- `useCompositeLoop.ts` - gates whether it builds the `clicksStr` cache-key component (`""` when this returns `false`, since the request never varies with clicks in that case).

## ActiveHit

```ts
export interface ActiveHit { x: number; y: number; progress: number }
```

`activeRippleHits`' output: one active click's raw 0..1 screen-content position (same basis `ClickSample` uses, NOT yet mapped to canvas px) and its progress 0..1 through its life. Structurally identical to `PixelHit`, kept as a distinct name for the coordinate space (pre-mapping).

## PixelHit

```ts
export interface PixelHit { x: number; y: number; progress: number }
```

What `drawRipplePreview` actually draws: one active click already mapped to canvas px by the caller. Structurally identical to `ActiveHit`, kept as a distinct name for the coordinate space (post-mapping).

## activeRippleHits

```ts
export function activeRippleHits(clicks: ClickSample[], now: number): ActiveHit[]
```

TS mirror of `export/fx/clickfx.rs::hits_at` - which clicks are "alive" at `now` and how far through their life each one is. Rust: `et >= e.t && et - e.t < life_ms` (inclusive start, EXCLUSIVE end) - the old inline version in `fxOverlay.ts` used an inclusive end (`dt > life`), an off-by-one against the export only visible on the single frame exactly at the boundary; fixed here since this is now the one place that math lives.

### Behaviors

- includes a click exactly at its start, excludes one exactly at its life boundary (the fixed off-by-one above), computes `progress` as elapsed/life, excludes a future click, keeps every simultaneously-alive click in order.

## rippleAlpha

```ts
export function rippleAlpha(progress: number, intensity: number): number
```

TS mirror of fx.wgsl's per-hit `a` (line 169) / `clickfx::fade_alpha` - fades 1→0 over the life, scaled by the user's intensity setting: `clamp01(1 - clamp01(progress)) * clamp01(intensity)`.

### Behaviors

- full intensity at progress 0; exactly 0.25 at progress 0.5, intensity 0.5 (matches the Rust `clickfx.rs` test `fade_alpha(0.5,0.5)==0.25`); 0 at progress 1; clamps progress/intensity outside 0..1.

## rippleRadiusPx

```ts
export function rippleRadiusPx(progress: number, oh: number): number
```

TS mirror of fx.wgsl line 172 (`FX_RIPPLE`) / `clickfx::ripple_radius`: ring radius grows linearly to `oh * 0.06` over the life. `oh` is the OUTPUT frame height in px - the export's `oh`; in the preview, the REAL canvas height, not the downscaled FX-request resolution (see `drawMirroredRipples`' doc for why that's the right basis here).

### Behaviors

- 0 at progress 0, `oh*0.06` at progress 1, half that at progress 0.5, clamped past 1.

## rippleThicknessPx

```ts
export function rippleThicknessPx(oh: number): number
```

TS mirror of fx.wgsl line 173 (`FX_RIPPLE` thickness): `max(oh * 0.006, 1)`.

### Behaviors

- `oh*0.006` for a normal size, floors at 1px for a small `oh`, exact at the floor boundary.

## shockwaveRadiusPx

```ts
export function shockwaveRadiusPx(progress: number, oh: number): number
```

TS mirror of fx.wgsl line 186 (`FX_SHOCKWAVE` radius) - the ring-draw half only. The shader ALSO warps nearby background pixels radially around the ring (lines 96-108, a screen-space UV refraction) - a per-pixel background resample this 2D canvas draw can't reproduce cheaply. `clickdraw.rs`'s own CPU fallback makes the identical trade (see its `ClickFxStyle::Shockwave` comment: "the CPU path can't do" the warp either) and compensates with the same fainter `SHOCKWAVE_ALPHA_MUL` this mirrors - so the ring alone, without the warp, is an established simplification in this codebase, not a new one.

### Behaviors

- 0 at progress 0, `oh*0.09` at progress 1, half that at progress 0.5.

## shockwaveThicknessPx

```ts
export function shockwaveThicknessPx(oh: number): number
```

TS mirror of fx.wgsl line 187 (`FX_SHOCKWAVE` thickness): `max(oh * 0.008, 1)`.

### Behaviors

- `oh*0.008` for a normal size, floors at 1px for a small `oh`.

## SHOCKWAVE_ALPHA_MUL

```ts
export const SHOCKWAVE_ALPHA_MUL = 0.5;
```

fx.wgsl line 188: the shockwave ring is additive at 0.5x the base alpha - deliberately fainter than Neon's 1.4x since the shader leans on the UV-warp (see `shockwaveRadiusPx`) for the rest of its punch.

## drawMirroredRipples

```ts
export function drawMirroredRipples(
  ctx: CanvasRenderingContext2D, clicks: ClickSample[], now: number,
  enabled: boolean, style: string, color: [number, number, number], intensity: number,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  fxW: number, fxH: number, canvasW: number, canvasH: number,
): void
```

Computes this tick's active, style-mirrored hits and draws them on the real canvas - the glue `useCompositeLoop.ts` calls straight from its rAF tick.

`enabled` (`clickfx.enabled`) is checked FIRST, before anything else - mirrors `fxOverlay.ts`'s `requestFxOverlay`, whose very first check is the same field, the master switch for the whole FX stack. A gate finding fixed a regression here: without this check, toggling "Click animations" off still drew ripple rings in the live preview (the export correctly draws none - `fx_state.rs`'s `render` returns early on `!enabled`).

`mapFn` (built there) outputs `fxW`x`fxH`-space px, the resolution the backend renders the spotlight/video-fx overlay at; scaling its result by `(canvasW/fxW, canvasH/fxH)` reproduces the IDENTICAL panel/zoom mapping at the real canvas's resolution instead - the mapping is linear/homogeneous in fxW/fxH (every intermediate quantity `mapFn` derives from - dx/dy/dw/dh/cx0/cy0/cw/ch - scales with it), so this is equivalent to recomputing that whole geometry a second time at full scale, without actually doing so. Radius/thickness then use the real canvas height directly for the same reason (see `rippleRadiusPx`'s doc).

No-ops via `drawRipplePreview` for a disabled clickfx, an unmirrored style, or no active hits - the (cheap) hit-list/mapping work still runs either way for the latter two, guarded by `stylesMirrored` up front so it's skipped entirely for a style this module doesn't draw.

### Behaviors

- no-op paths (disabled clickfx, unmirrored style, empty click list) never touch `ctx` at all, including when an active click and a mirrored style would otherwise draw - asserted with a throwing Proxy stub, not just a return-value check.

### Used by

- `useCompositeLoop` (`src/editor/hooks/useCompositeLoop.ts`) - called right after the base `drawPreview` composite and BEFORE the cached FX-overlay blit, so an active spotlight's dim composites on top of a ripple like every other preview element.

## drawRipplePreview

```ts
export function drawRipplePreview(
  ctx: CanvasRenderingContext2D, hits: PixelHit[], style: string,
  color: [number, number, number], intensity: number, oh: number,
): void
```

Draws every active, style-mirrored hit onto `ctx` (already the real canvas, full resolution). No-ops for a style not in `stylesMirrored`, an empty hit list, or a non-positive `oh`.

Ripple blends normally (mirrors fx.wgsl's `mix`, an over-composite); Shockwave is additive (`globalCompositeOperation = "lighter"`, mirrors the shader's `color = color + ...`). Each ring's soft triangular coverage falloff (`(thick - |d-radius|) / thick`, clamped 0..1 - the exact per-pixel shape fx.wgsl computes) is reproduced losslessly via a 3-stop radial gradient: canvas gradients interpolate LINEARLY between stops, which is exactly what that falloff already is (a linear ramp up to the ring, then back down) - no per-pixel loop needed on the 2D canvas. The inner stop's offset is clamped to `>= 0` and the peak stop's to `>= ` the inner one, so the ripple's very first ~10% of life (`radius < thick`) degrades to a soft disc instead of an inverted gradient - matching what the shader itself produces there (`cov(0) > 0` in that regime).

### Behaviors

- no-op paths (unmirrored style, empty hits, non-positive `oh`) never touch `ctx` at all - asserted with a throwing Proxy stub, not just a return-value check.

### Used by

- `drawMirroredRipples` (this file).
