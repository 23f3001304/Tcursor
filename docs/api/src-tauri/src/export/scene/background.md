# src-tauri/src/export/scene/background.rs

Rasterizes the export background layer (solid colour, linear gradient at any angle, or image stub) into a raw BGRA pixel buffer. This is the only file that touches `Background` at draw time; callers receive a flat buffer and never inspect which variant was used. `render` is pure and deterministic - identical inputs always produce identical output. `build` is the user-facing entry point: it turns the persisted `BackgroundSettings` (`settings::background`) into that same buffer, optionally softened by a one-time blur pass.

## build

```rust
pub fn build(settings: &BackgroundSettings, mesh_jpg: &[u8], w: u32, h: u32) -> Vec<u8>
```

Builds the static background buffer `FrameRenderer` composites under the screen every frame. Called once per export/preview build (`FrameRenderer::new`) and again from `reload_edit` ONLY when background settings changed - never per output frame - so the ffmpeg-backed `Mesh` decode and the blur pass are both cheap in practice despite not being optimized for repeated calls.

### Inputs

- `settings: &BackgroundSettings` - the user's background choice (`kind` + solid/gradient colors + `blur`). *Why a settings struct rather than `Background` directly:* `Background` (this file's render-time enum) has no `Mesh` variant - it's a separate concept unique to this entry point, so `build` maps `BackgroundKind` to whichever `Background` variant (or ffmpeg decode) actually produces it.
- `mesh_jpg: &[u8]` - the bundled default background image bytes (`render::BG_MESH`), used only when `settings.kind == Mesh`. *Why passed in rather than included here:* keeps this file free of an `include_bytes!` dependent on the caller's asset layout; `render/mod.rs` already owns that constant for the fallback path.
- `w: u32`, `h: u32` - output frame dimensions in pixels.

### Returns

`Vec<u8>` of length `w * h * 4`, BGRA byte order - same contract as `render`.

### Implementation

1. Dispatch on `settings.kind`: `Mesh` calls `pipeline::ffio::decode_image(mesh_jpg, w, h)` (spawns ffmpeg), falling back to `render(&Background::default(), w, h)` on decode failure (today's exact fallback behavior, unchanged). `Solid`/`Gradient` build the matching `Background` variant from the settings' RGB fields and call `render` directly - no subprocess.
2. If `settings.blur > 0.0`, run the two-pass box blur (`blur`) over the buffer in place.

### Behaviors

- `build_solid_matches_direct_render` - `Solid` output is byte-identical to calling `render` directly with the same color; `mesh_jpg` is irrelevant (passed as `&[]`) since `Solid` never reads it.
- `build_zero_blur_is_a_no_op` - `blur: 0.0` produces output identical to `render` with no blur step, confirming zero is a true no-op (back-compat: a doc/config saved before `blur` existed renders unchanged).

### Used by

- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`, `FrameRenderer::reload_edit`) - the only caller; `reload_edit` gates the call behind a `BackgroundSettings` equality check so unrelated edits never re-decode the mesh.

## render

```rust
pub fn render(bg: &Background, w: u32, h: u32) -> Vec<u8>
```

Allocates a `w x h` BGRA buffer and fills it according to `bg`.

### Inputs

- `bg: &Background` - the background variant to rasterize. *Why:* the enum unifies all background types so the compositor calls a single entry point rather than dispatching per variant.
- `w: u32` - output frame width in pixels. *Why:* determines buffer size and the per-row stride in `fill`.
- `h: u32` - output frame height in pixels. *Why:* determines buffer size and the gradient projection axis extent.

### Returns

`Vec<u8>` of length `w * h * 4`, BGRA byte order (B at offset 0), alpha channel fixed at 255. The exporter composites all other layers on top of this buffer and never reads its alpha channel.

### Implementation

1. Allocate `buf` as `w * h * 4` zero bytes.
2. Dispatch on `bg` variant:
   - `Solid(c)` - call `fill` with a closure returning the constant `c`. O(w*h), no conditionals inside the loop.
   - `Image(_)` - M2b stub: the image library is deferred to M4. Falls through to `fill` with a hard-coded dark colour `Rgb { r:24, g:24, b:30 }` so exports are never broken when this variant is selected.
   - `Gradient { from, to, angle_deg }` - convert `angle_deg` to radians; derive direction `(dx, dy)` = `(cos, sin)`; compute `max` as the maximum possible signed projection across the frame (sum of absolute per-axis extents); call `fill` with `lerp(from, to, t)` where `t = |(x*dx + y*dy)| / max`. *Why normalise by `max`:* guarantees the gradient always spans the full 0..1 range regardless of angle - a 45-degree gradient runs corner to corner, not just to the midpoint.
3. Return `buf`.

### Behaviors

- `solid_fills_bgra` - a 2x2 solid buffer with `Rgb { r:10, g:20, b:30 }` has `[30, 20, 10, 255]` at offset 0, confirming BGRA byte order and alpha.
- `gradient_differs_corner_to_corner` - a 0-degree (horizontal) black-to-white gradient over 4 pixels has a lighter rightmost pixel, confirming the ramp spans the whole width.
