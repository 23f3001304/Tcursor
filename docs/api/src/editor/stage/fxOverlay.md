# src/editor/stage/fxOverlay.ts

Builds the backend FX-overlay request (spotlight + video-fx, and click ripples for an unmirrored style) from the current preview state and calls the `preview_fx_overlay` Tauri command, which renders it with the same renderer the export would use for that frame (`with_fx` → `select_fx`). Returning `null` early - FX disabled, or nothing active - skips the IPC round-trip entirely.

**Click ripples are conditional on the active style (sweep-2).** They used to ALWAYS be part of this request, mapped through `mapFn` and sent as `hits`, refreshed only at this call's `FX_BUCKET_MS` cadence with single-flight gating - the highest-frequency element on screen getting the coarsest, laggiest update rate. `ripplePreview.ts`'s `stylesMirrored` styles (Ripple - the default - plus Shockwave and Pulse) now draw straight on the preview canvas every `useCompositeLoop` tick instead, and are EXCLUDED from this request (`overlayNeedsClicks` returns `false` for them). The other three click-fx styles (Glow/Neon/Particles) are NOT mirrored, so `overlayNeedsClicks` returns `true` for them and this request keeps building their `hits` exactly as before this pass - laggy at `FX_BUCKET_MS` cadence, but still visible in the live preview, not a silent regression to nothing.

This file holds no effect *logic* otherwise: everything it does is resolve which spotlight/video-fx/click values to send.

### Behaviors

- `includes click hits for an UNMIRRORED style (glow) - falls back to the overlay exactly as before` / `drops an unmirrored-style click once it passes 600ms` - the export's 600ms lifetime (`ripplePreview.ts`'s `RIPPLE_LIFE_MS`, via `activeRippleHits`), not the old 500ms bug.
- `excludes click hits for a MIRRORED style (ripple)` / `... (shockwave)` - `ripplePreview.ts` draws those client-side instead.
- `excludes click hits when the style is none` / `still draws the spotlight regardless of the click style setting`.
- `triggers a backend call from an unmirrored-style click ALONE, spotlight off` - an unmirrored style still pays the pre-sweep-2 request cost when the user clicks.
- `returns null with no backend call for a mirrored-style click ALONE, spotlight off` - the actual win: a Ripple/Shockwave/Pulse click no longer triggers this "hottest command" (`useCompositeLoop.md`) at all.
- `excludes click hits for Pulse, mirrored client-side since the click-fx look pass` - Pulse joined `stylesMirrored` when the click styles were reworked, so it must not be drawn twice (once here, once on the canvas).
- `returns null with no backend call when nothing is active at all` / `renders nothing at all when fx are disabled, spotlight included`.

## FxCamRect

```ts
export type FxCamRect = { rect: [number, number, number, number]; radius: number } | null;
```

The active camera panel's rect in the same FX-render pixel space as everything else passed to `requestFxOverlay` (min_x, min_y, max_x, max_y), plus its corner radius, or `null` when no camera panel is shown this frame. Mirrors the export's `Spot.cam_rect`/`Spot.cam_radius`.

### Used by

- `src/editor/stage/fxOverlay.ts` (`requestFxOverlay`) - the `camRect` parameter's type.
- `src/editor/hooks/useCompositeLoop.ts` - computes this from `frameLayout.cam` (the webcam PiP is a fixed, unzoomed overlay, so its rect is the layout fraction applied directly, not the zoom-crop projection `mapFn` uses).

## requestFxOverlay

```ts
export async function requestFxOverlay(
  ow: number, oh: number,
  clicks: ClickSample[], now: number,
  cursorPx: [number, number] | null,
  resolved: ResolvedSpotlight | null,
  clickfx: ClickFxSettings,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  screenScale: number,
  camRect: FxCamRect,
): Promise<string | null>
```

### Inputs

- `ow`, `oh` - the overlay's render resolution (the caller may request less than the full canvas size to cut backend cost; the returned image upscales cleanly on blit).
- `clicks`, `now` - the click track and current time; used to build hits ONLY when `overlayNeedsClicks(clickfx.style)` is true (`ripplePreview.ts`) - a mirrored style's clicks never reach this function's `hits` array at all.
- `cursorPx` - the zoom-projected cursor position in the *same* `ow`x`oh` space, or `null`.
- `resolved` - the spotlight **already resolved** by the caller's own `resolveSpotlight(...)` call for this exact frame; `null`/inactive skips the spotlight entirely. This function does not call `resolveSpotlight` itself - `resolveSpotlight` mutates a `SpotlightSimState` in place (region handoff/transition tracking), so it must run exactly once per composite tick, not once here and once in the caller's cache-key computation (that double call used to double-advance the same stateful sim for no benefit, since both calls always used the same `now`).
- `clickfx` - the recording's click-FX settings (style/color/intensity/enabled/`spotlight_dim_camera`).
- `mapFn` - projects a 0..1 screen-content point into the same `ow`x`oh` pixel space as `ow`/`oh` (the caller's zoom-crop projection); used to place each click hit when `overlayNeedsClicks` is true.
- `screenScale` - the screen panel's height as a fraction of the FX render height (`= layout.screen[3]`). `spotRadius`/`spotFeather` are pre-multiplied by it so the spotlight is sized to the screen panel, matching the export's `fx_state_at` (`scene.screen.h / oh`) - without it the preview spotlight would be a fixed fraction of the whole frame (layout-agnostic).
- `camRect: FxCamRect` - the camera panel's rect this frame, or `null` when no camera panel is shown. *Why needed even though the export derives it from `scene.camera`:* the preview has no `Scene`, so the caller must resolve and pass the equivalent rect itself.

### Returns

`Promise<string | null>` - a `data:image/png;base64,...` URL of the transparent overlay, or `null` when nothing is active (no backend call is made in that case). **A `null` RESOLUTION and a REJECTED promise are deliberately distinct**: `null` means "nothing to draw at this key" - a valid, latchable terminal state the caller stops re-requesting until the key changes - while a rejection means the backend call itself failed and must stay retryable. See `fxResponseAction` (`fxCacheKey.ts`), which the caller uses to act on this distinction.

### Implementation

0. Return `null` immediately when `clickfx.enabled` is false. *Why this is the very first check:* `enabled` is the master switch for the whole FX stack, spotlight included - the export's `fx_state::render` returns before drawing anything when it is off. Previously `enabled` only guarded the click-hit loop, so a recording with FX switched off still showed a spotlight while editing and exported without one.
1. Build `hits` from `activeRippleHits(clicks, now)` (`ripplePreview.ts` - the export's exact 600ms lifetime/boundary math), mapping each active hit's position through `mapFn` - but ONLY when `overlayNeedsClicks(clickfx.style)` is true; otherwise `hits` stays `[]` (a mirrored style, or `"none"`).
2. `spotActive = resolved && resolved.alpha > 0.001 && cursorPx`, from the already-resolved `resolved` param (no `resolveSpotlight` call here).
3. If neither `hits` nor an active spotlight exist, return `null` without calling the backend.
4. Otherwise assemble `FxOverlayParams` (`hits` + spotlight fields, with `spotRadius`/`spotFeather` scaled by `screenScale`); when `camRect` is non-null, also set `camRect`/`camRadius`/`dimCamera` (from `clickfx.spotlight_dim_camera`) on the params. `await previewFxOverlay(params)`; on an IPC error, log it (dev-only) and **rethrow** - it must surface as a rejected promise, not collapse into the same `null` that means "nothing to draw" (see Returns above).
