# src/editor/stage/fxOverlay.ts

Builds the backend FX-overlay request (spotlight + click effects) from the current preview state and calls the `preview_fx_overlay` Tauri command, which runs the exact same GPU/CPU shader pipeline as the export. Returning `null` early (no active clicks and no active spotlight) skips the IPC round-trip entirely.

## requestFxOverlay

```ts
export async function requestFxOverlay(
  ow: number, oh: number,
  clicks: ClickSample[], now: number,
  cursorPx: [number, number] | null,
  spotlight: SpotlightInput | null,
  clickfx: ClickFxSettings,
  mapFn: (fx: number, fy: number) => [number, number] | null,
  sim: SpotlightSimState,
  screenScale: number,
): Promise<string | null>
```

### Inputs

- `ow`, `oh` - the overlay's render resolution (the caller may request less than the full canvas size to cut backend cost; the returned image upscales cleanly on blit).
- `clicks`, `now` - the click track and current time; clicks within `RIPPLE_MS` (500ms) of `now` become active hits.
- `cursorPx` - the zoom-projected cursor position in the *same* `ow`x`oh` space, or `null`.
- `spotlight` - resolved via `resolveSpotlight`; `null`/inactive skips the spotlight entirely.
- `clickfx` - the recording's click-FX settings (style/color/intensity/enabled).
- `mapFn` - projects a 0..1 screen-content point into the same `ow`x`oh` pixel space as `ow`/`oh` (the caller's zoom-crop projection); used to place each click hit.
- `sim` - the persistent `SpotlightSimState` (layer/handoff state) `resolveSpotlight` reads and mutates in place, mirroring the export's `SpotlightSim`.
- `screenScale` - the screen panel's height as a fraction of the FX render height (`= layout.screen[3]`). `spotRadius`/`spotFeather` are pre-multiplied by it so the spotlight is sized to the screen panel, matching the export's `fx_state_at` (`scene.screen.h / oh`) - without it the preview spotlight would be a fixed fraction of the whole frame (layout-agnostic).

### Returns

`Promise<string | null>` - a `data:image/png;base64,...` URL of the transparent overlay, or `null` when nothing is active (no backend call is made in that case).

### Implementation

1. Build the active click `hits` (pixel position + 0..1 progress) by mapping each recent click through `mapFn`, skipping any style-disabled or off-crop (`mapFn` returned `null`) clicks.
2. Resolve the spotlight via `resolveSpotlight(spotlight, now, sim)`; `spotActive = resolved && alpha > 0.001 && cursorPx`.
3. If neither hits nor an active spotlight exist, return `null` without calling the backend.
4. Otherwise assemble `FxOverlayParams` (hits + spotlight fields, with `spotRadius`/`spotFeather` scaled by `screenScale`) and `await previewFxOverlay(params)`, catching and logging (dev-only) any IPC error as `null`.
