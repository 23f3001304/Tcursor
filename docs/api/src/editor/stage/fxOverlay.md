# src/editor/stage/fxOverlay.ts

Builds the backend FX-overlay request (spotlight + click effects) from the current preview state and calls the `preview_fx_overlay` Tauri command, which renders it with the same renderer the export would use for that frame (`with_fx` → `select_fx`). Returning `null` early - FX disabled, or no active clicks and no active spotlight - skips the IPC round-trip entirely.

This file holds no effect *logic*: everything it does is resolve which values to send. The one thing it must keep in step with Rust by hand is the click lifetime constant below.

### Behaviors

- `uses the export's 600ms click lifetime, not 500ms` / `drops a click once it passes 600ms` - a 550ms-old click is still alive with `progress = 550/600`, and a 650ms-old one is gone. Pins `RIPPLE_MS` to the export's `LIFE_MS`.
- `renders nothing at all when fx are disabled, spotlight included` - `clickfx.enabled: false` returns `null` and makes no IPC call, even with an active spotlight and a live click.
- `still draws the spotlight when the click style is none` - `"none"` suppresses only the hits; the spotlight is still requested.

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
- `clicks`, `now` - the click track and current time; clicks within `RIPPLE_MS` of `now` become active hits. **`RIPPLE_MS` must stay equal to the export's `LIFE_MS` (600ms, `fx_state.rs`)** - a hit's `progress` is `elapsed / lifetime` and drives both ring radius and fade alpha, so when this constant was 500 every preview ring was ~20% further along than the export's at the same instant (smaller, fainter) and vanished 100ms early.
- `cursorPx` - the zoom-projected cursor position in the *same* `ow`x`oh` space, or `null`.
- `resolved` - the spotlight **already resolved** by the caller's own `resolveSpotlight(...)` call for this exact frame; `null`/inactive skips the spotlight entirely. This function does not call `resolveSpotlight` itself - `resolveSpotlight` mutates a `SpotlightSimState` in place (region handoff/transition tracking), so it must run exactly once per composite tick, not once here and once in the caller's cache-key computation (that double call used to double-advance the same stateful sim for no benefit, since both calls always used the same `now`).
- `clickfx` - the recording's click-FX settings (style/color/intensity/enabled/`spotlight_dim_camera`).
- `mapFn` - projects a 0..1 screen-content point into the same `ow`x`oh` pixel space as `ow`/`oh` (the caller's zoom-crop projection); used to place each click hit.
- `screenScale` - the screen panel's height as a fraction of the FX render height (`= layout.screen[3]`). `spotRadius`/`spotFeather` are pre-multiplied by it so the spotlight is sized to the screen panel, matching the export's `fx_state_at` (`scene.screen.h / oh`) - without it the preview spotlight would be a fixed fraction of the whole frame (layout-agnostic).
- `camRect: FxCamRect` - the camera panel's rect this frame, or `null` when no camera panel is shown. *Why needed even though the export derives it from `scene.camera`:* the preview has no `Scene`, so the caller must resolve and pass the equivalent rect itself.

### Returns

`Promise<string | null>` - a `data:image/png;base64,...` URL of the transparent overlay, or `null` when nothing is active (no backend call is made in that case). **A `null` RESOLUTION and a REJECTED promise are deliberately distinct**: `null` means "nothing to draw at this key" - a valid, latchable terminal state the caller stops re-requesting until the key changes - while a rejection means the backend call itself failed and must stay retryable. See `fxResponseAction` (`fxCacheKey.ts`), which the caller uses to act on this distinction.

### Implementation

0. Return `null` immediately when `clickfx.enabled` is false. *Why this is the very first check:* `enabled` is the master switch for the whole FX stack, spotlight included - the export's `fx_state::render` returns before drawing anything when it is off. Previously `enabled` only guarded the click-hit loop, so a recording with FX switched off still showed a spotlight while editing and exported without one.
1. Build the active click `hits` (pixel position + 0..1 progress) by mapping each recent click through `mapFn`, skipping any off-crop (`mapFn` returned `null`) clicks; skipped entirely when the style is `"none"`.
2. `spotActive = resolved && resolved.alpha > 0.001 && cursorPx`, from the already-resolved `resolved` param (no `resolveSpotlight` call here).
3. If neither hits nor an active spotlight exist, return `null` without calling the backend.
4. Otherwise assemble `FxOverlayParams` (hits + spotlight fields, with `spotRadius`/`spotFeather` scaled by `screenScale`); when `camRect` is non-null, also set `camRect`/`camRadius`/`dimCamera` (from `clickfx.spotlight_dim_camera`) on the params. `await previewFxOverlay(params)`; on an IPC error, log it (dev-only) and **rethrow** - it must surface as a rejected promise, not collapse into the same `null` that means "nothing to draw" (see Returns above).
