# src-tauri/src/export/preview_fx.rs

The editor preview's FX overlay (spotlight + click effects) as a standalone **transparent PNG**, which the canvas loop (`useCompositeLoop`) blits over the base frame it composites in JS. The frontend has already resolved every value - the spotlight centre/radius/feather/alpha and the click hits are all in FX-canvas pixels - so this command builds an `FxState` straight from the params (no events/actions/scene) and runs the exact `CpuFx` primitives the export uses. It is a pure function of its arguments: no `folder`, no `PreviewSession`, no decode.

Splitting it out of `preview.rs`/`preview_track.rs` keeps each file focused and under the 200-line budget.

## preview_fx_overlay

```rust
#[tauri::command]
pub fn preview_fx_overlay(
    ow: u32, oh: u32,
    style: String, color: [u8; 3], intensity: f32, hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>, spot_cy: Option<f32>, spot_dim: Option<f32>,
    spot_radius: Option<f32>, spot_feather: Option<f32>, spot_alpha: Option<f32>,
    spot_mode: Option<String>, spot_tint: Option<[u8; 3]>, spot_t: Option<f32>,
    video_mode: Option<String>, video_alpha: Option<f32>, video_t: Option<f32>,
) -> Result<String, String>
```

Renders the resolved FX at one preview frame and returns a `data:image/png;base64,...` overlay. Registered in `lib.rs`; the frontend calls it via `previewFxOverlay` (`src/lib/ipc.ts`) → `requestFxOverlay` (`src/editor/fxOverlay.ts`). **This command being absent is exactly why the spotlight never appeared in the preview:** the invoke rejected as an unregistered command, `fxOverlay.ts` caught the error and returned `null`, so no overlay was ever blitted.

### Inputs (what, and why it is needed)

- `ow` / `oh: u32` - the FX render size in pixels (the preview canvas × `FX_SCALE`). *Why:* the overlay is produced at reduced resolution and upscaled on blit; the frontend maps hit/cursor coordinates into this same space.
- `style: String`, `color: [u8; 3]`, `intensity: f32` - the click-FX look (lowercase style name e.g. `"ripple"`, RGB, 0..1 strength). *Why:* drives the click ripple/glow primitives, identical to `ClickFxSettings`.
- `hits: Vec<[f32; 3]>` - active click hits as `[x, y, progress]` in FX pixels. *Why:* the frontend already resolved which clicks are live and where, through the current zoom crop.
- `spot_*: Option<...>` - the resolved spotlight: centre `spot_cx/cy`, `spot_dim`, screen-scaled `spot_radius`/`spot_feather`, `spot_alpha`, lowercase `spot_mode`, `spot_tint`, and `spot_t` (seconds, for the breathing phase). *Why:* the region-override + fade resolution happens in `spotlightPreview.ts` (the mirror of `SpotlightSim`), so the backend just draws it. `None` (or `spot_alpha <= 0`) means no spotlight this frame.
- `video_*: Option<...>` - an optional full-frame video FX (mode/alpha/seconds). *Why:* symmetry with the export's `VideoFx`; unused by the current preview (always `None`).

### Returns

`Result<String, String>` - a PNG data URL of the straight-alpha overlay, or an error string if PNG encoding fails. An empty effect yields a fully transparent PNG (harmless; the frontend skips the call when nothing is active).

### Implementation

1. Assemble an `FxState` from the params: build `Spot`/`VideoFx` only when their alpha is present and `> 0`, map the lowercase enum strings (`click_style_of`/`spot_mode_of`/`video_mode_of`), and turn `hits` into `FxHit`s.
2. **Alpha reconstruction.** `CpuFx` composites *onto an opaque frame* (it multiply-dims and additively tints in place), so there is no source alpha to read back. Render the same `FxState` twice - once over solid **black**, once over solid **white** - then invert the over-composite per pixel: `white - black = 255·(1 - a)` per channel, so `a = 1 - (white - black)/255` (take the strongest-touched channel), and the straight colour is `black / a`. Pixels the effect never touched come out fully transparent, dim areas resolve to black-with-alpha (so the blit darkens the base), and additive click pixels resolve to bright-colour-with-alpha.
3. `png_encode` the reconstructed BGRA overlay (it preserves the alpha channel) and base64 into a data URL, reusing `preview.rs`'s helpers.

*Why `CpuFx` and not `select_fx`:* the preview overlay is small and requested ~25×/sec; creating a GPU device per call would thrash. `CpuFx` (`fxdraw.rs`) is also the reference the wgpu `fx.wgsl` shader was aligned to, so the look matches the export.
