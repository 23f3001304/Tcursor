# src-tauri/src/export/scene/background.rs

Rasterizes the export background layer (a bundled wallpaper, a solid colour, a two- or three-stop linear gradient at any angle, or the user's own imported image/video) into a raw BGRA pixel buffer. This is the only file that touches `Background` at draw time; callers receive a flat buffer and never inspect which variant was used. `render` is pure and deterministic - identical inputs always produce identical output. `build` is the user-facing entry point: it turns the persisted `BackgroundSettings` (`settings::background`) into that same buffer, optionally softened by a one-time blur pass.

## build

```rust
pub fn build(settings: &BackgroundSettings, mesh_jpg: &[u8], w: u32, h: u32, project_dir: &Path) -> Vec<u8>
```

Builds the static background buffer `FrameRenderer` composites under the screen every frame. Called once per export/preview build (`FrameRenderer::new`) and again from `reload_edit` ONLY when background settings changed - never per output frame - so the ffmpeg-backed `Mesh` decode and the blur pass are both cheap in practice despite not being optimized for repeated calls.

### Inputs

- `settings: &BackgroundSettings` - the user's background choice (`kind` + solid/gradient colors + `blur`). *Why a settings struct rather than `Background` directly:* `Background` (this file's render-time enum) has no `Mesh` variant - it's a separate concept unique to this entry point, so `build` maps `BackgroundKind` to whichever `Background` variant (or ffmpeg decode) actually produces it.
- `mesh_jpg: &[u8]` - the LEGACY bundled background image bytes (`render::BG_MESH`, `assets/backgrounds/bg.jpg`), used when `settings.kind == Mesh` and `settings.mesh` names no wallpaper this build has. *Why passed in rather than included here:* keeps this file free of an `include_bytes!` dependent on the caller's asset layout; `render/mod.rs` already owns that constant. The wallpaper LIBRARY's own bytes are not passed in - they come from `settings::wallpapers`, which owns them.
- `w: u32`, `h: u32` - output frame dimensions in pixels.
- `project_dir: &Path` - the recording's folder, which an `Image`/`Video` asset path is relative to. *Why a parameter rather than an absolute path in the settings:* projects have to stay portable, so `BackgroundSettings.asset` stores `background/<file>` and nothing else; this is where that gets resolved (through `settings::bg_asset::asset_path`, which refuses anything absolute or climbing out with `..`). Irrelevant for every other kind, which is why the tests pass `Path::new(".")`.

### Returns

`Vec<u8>` of length `w * h * 4`, BGRA byte order - same contract as `render`.

### Implementation

1. Dispatch on `settings.kind`:
   - `Mesh` resolves `settings.mesh` through `settings::wallpapers::wallpaper_by_id`. A hit decodes that wallpaper with `ffio::decode_image_cover` (aspect preserved, overflow cropped). A miss - the empty id every pre-library doc carries, or an id this build does not know - decodes the legacy `mesh_jpg` with the stretching `ffio::decode_image`, falling back to `render(&Background::default(), w, h)` if even that fails (today's exact fallback behavior, unchanged). A wallpaper whose own decode fails ALSO falls back to the legacy image rather than to the flat default gradient: a broken ffmpeg should cost the user the wallpaper they picked, not their whole background.
   - `Solid`/`Gradient` build the matching `Background` variant from the settings' RGB fields (including `gradient_mid`) and call `render` directly - no subprocess.
   - `Image`/`Video` resolve `settings.asset` under `project_dir` and decode the file cover-fit (`ffio::decode_file_cover`, the path-input twin of the wallpaper decode, so an imported still is framed exactly like a bundled one). For a `Video` that is its FIRST frame: the moving picture is `pipeline::bg_pipe`'s job in the export and the TS preview's in the editor, and this frame is what the Rust preview commands show and what the export falls back to if the stream dies. A missing file, an unreadable one, or a failed decode falls back to `base()` - the user's chosen wallpaper, or the legacy mesh - and logs ONCE (`warn_asset`). A broken import must never cost the whole background, let alone fail an export.
2. If `settings.blur > 0.0`, run the two-pass box blur (`blur`) over the buffer in place. Note this is a one-off pass over the STATIC buffer, so on a `Video` background it reaches the first frame only, never the streamed ones; `BackgroundPanel` therefore hides the Blur slider while `kind == Video` rather than leaving a control that visibly does nothing. Nothing here changes either way.
3. `apply_dim(&mut buf, settings.dim_clamped())` - last, so the dim sits over the blurred result the way a black overlay would.

### Behaviors

- `a_middle_stop_lands_at_the_ramp_midpoint` - with a middle stop, the pixel at `t = 0.5` is EXACTLY that colour, and the two halves ramp between the right pairs of stops.
- `no_middle_stop_is_the_two_stop_ramp_unchanged` - `mid: None` is bit-identical to the pre-middle-stop renderer at a non-trivial angle.
- `build_passes_the_middle_stop_through_to_the_renderer` - `BackgroundSettings.gradient_mid` actually reaches `render`, not just `Background`.
- `build_solid_matches_direct_render` - `Solid` output is byte-identical to calling `render` directly with the same color; `mesh_jpg` is irrelevant (passed as `&[]`) since `Solid` never reads it.
- `build_zero_blur_is_a_no_op` - `blur: 0.0` produces output identical to `render` with no blur step, confirming zero is a true no-op (back-compat: a doc/config saved before `blur` existed renders unchanged).
- `dim_multiplies_every_channel_and_leaves_alpha_alone` - `apply_dim` is `src * (1 - dim)` on BGR, alpha untouched; `0.0` changes not one byte and `1.0` is black.
- `build_applies_the_clamped_dim_to_a_solid` - the dim `build` applies is `dim_clamped`, not the raw field.
- `a_missing_asset_falls_back_to_the_base_wallpaper_instead_of_failing` - an `Image` whose file is gone still yields a full-size buffer.
- `video_source_is_only_the_video_kind_with_a_real_file` - `Image`, `Mesh` (with an asset kept for later) and a missing file all yield `None`.

Both `Mesh` paths shell out to ffmpeg, so neither is exercised by this file's tests. They are covered where the decode itself is (`pipeline::ffio_tests`, which skips without ffmpeg) plus `settings::wallpapers`' id-lookup tests, which are what decide between the two.

### Used by

- `src-tauri/src/export/render/bg.rs` (`build_bg`) - the only caller, supplying the embedded legacy mesh and the project folder; `FrameRenderer::reload_edit` gates it behind a `BackgroundSettings` equality check so unrelated edits never re-decode anything.
- `src-tauri/src/settings/wallpapers.rs` - supplies the wallpaper bytes and the gradient presets this renders.

## asset_file

```rust
fn asset_file(settings: &BackgroundSettings, project_dir: &Path) -> Option<PathBuf>
```

The absolute path of the imported asset, for the two kinds that have one. `None` for every other kind, and for an asset whose file is not there.

## video_source

```rust
pub fn video_source(settings: &BackgroundSettings, project_dir: &Path) -> Option<PathBuf>
```

The file the export should open a decode STREAM on: a `Video` background whose asset really exists. `None` for an image (which needs no stream), for a wallpaper/colour (even one that still remembers an asset for later), and for a missing file.

This is also what decides `Compositor::set_bg_dynamic`, so the "does the background move?" question has exactly one answer in the codebase.

## apply_dim

```rust
pub fn apply_dim(buf: &mut [u8], dim: f32)
```

Darken a BGRA buffer by `dim` (0..1), which is exactly a black overlay at that alpha: `out = round(src * (1 - dim))`, alpha untouched. `src/editor/stage/canvas/stageBg.ts` paints the same formula over the preview's own video draw, so preview and export agree.

*Why a multiply rather than a real composite:* a black overlay at alpha `a` IS `src * (1 - a)`, and this way there is no second buffer. `dim == 0` returns immediately without touching a byte, which keeps the default path free - for a video background this runs once per FRAME, not once per build.

## warn_asset

```rust
fn warn_asset(what: &dyn std::fmt::Display)
```

Say ONCE per process that a background asset could not be used. Once, because this sits on a path that can run per frame and a broken import would otherwise flood the log.

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
   - `Gradient { from, mid, to, angle_deg }` - convert `angle_deg` to radians; derive direction `(dx, dy)` = `(cos, sin)`; project all FOUR frame corners `(0, wf*dx, hf*dy, wf*dx+hf*dy)` (where `wf = w-1`, `hf = h-1`) onto that direction and take `pmin`/`pmax` across them; `range = (pmax - pmin).max(1e-6)`; compute `t = (((x*dx + y*dy) - pmin) / range).clamp(0, 1)`. With `mid: None` the pixel is `lerp(from, to, t)`, exactly as before. With `mid: Some(m)` the ramp is two half-length lerps meeting at `t = 0.5`: `lerp(from, m, t*2)` below it, `lerp(m, to, (t-0.5)*2)` above. *Why a midpoint rather than a positionable stop:* a stop position would be a fourth gradient field to persist, expose and validate for a control users do not ask for; splitting the ramp in half is what every preset in `GRADIENT_WALLPAPERS` wants. *Why normalize against the true corner range rather than `.abs()` of the projection:* `.abs()` mirror-folds any angle whose projection goes negative for part of the frame - including the DEFAULT 135deg - putting a crease of the `from` color along the fold line instead of a monotonic corner-to-corner ramp. Projecting all four corners (not just one) and using the actual `[pmin, pmax]` span is correct at every angle, including ones where the extremes aren't at `(0,0)`/`(w,h)`.
3. Return `buf`.

### Behaviors

- `solid_fills_bgra` - a 2x2 solid buffer with `Rgb { r:10, g:20, b:30 }` has `[30, 20, 10, 255]` at offset 0, confirming BGRA byte order and alpha.
- `gradient_differs_corner_to_corner` - a 0-degree (horizontal) black-to-white gradient over 4 pixels has a lighter rightmost pixel, confirming the ramp spans the whole width.
- `gradient_at_135deg_is_a_true_monotonic_ramp_not_mirror_folded` - at the DEFAULT 135deg angle (100x100, black-to-white): walking the anti-diagonal (top-right -> center -> bottom-left) is strictly darker-to-lighter, and the two off-axis corners `(0,0)`/`(99,99)` are equal (both sit at the ramp's midpoint) - the regression case for the old `.abs()`-normalized formula, which instead put a crease of `from` along that diagonal.
