# src-tauri/src/export/render/mod.rs

Reusable per-frame renderer that owns all per-export setup except the raw decoders and the encoder sink. Extracted from `exporter.rs` so the same compositing code can serve both the export loop and the preview engine (Task 2). The exporter calls `FrameRenderer::new` once, then `step_camera` + `composite_at` once per output frame. The preview engine fast-forwards `step_camera` from 0 to a target time (cheap math, no decode) and composites once.

## OUT_FPS

```rust
pub const OUT_FPS: u64 = 60
```

Constant output frame rate (60 fps). Defined here so both `exporter.rs` and future preview code share a single source of truth for the encode rate.

## OUT_STEP_MS

```rust
pub const OUT_STEP_MS: f32 = 1000.0 / OUT_FPS as f32
```

The exact frame period of `OUT_FPS` in milliseconds - 16.666667, what every 60fps caller hands `step_camera` as its `dt_ms`. Deliberately **not** `1000 / OUT_FPS` (which is 16 in integer math): that rounding is the whole thing this constant exists to avoid. A frame loop running at some other rate (the exporter, whose `out_fps` is settings-resolved and can be 30) computes `1000.0 / out_fps as f32` itself rather than using this.

`RenderMeta` and `FramePose` - the two small data-only types `FrameRenderer::new`/`step_camera` return - now live in the sibling `meta.rs` (split out purely for size) and are re-exported here (`pub use meta::{FramePose, RenderMeta};`) so this file's own callers are unaffected. See `docs/api/src-tauri/src/export/render/meta.md`.

## FrameRenderer

```rust
pub struct FrameRenderer { /* private fields */ }
```

Owns all per-export setup that is not a raw decoder or encoder sink: event log, settings, layout, zoom regions, background pixels, compositor, FX renderer, camera sim, cursor state, cursor sprite set, and action log. Initialized once per export/preview; then drives per-frame rendering via `step_camera` and `composite_at`.

Private fields include:
- `settings: Settings` - cloned from the edit doc; drives all effect toggles per frame.
- `cfg: ZoomConfig` - derived from `settings.zoom`; used by `CameraSim::step`.
- `layout: Layout` - output canvas dimensions and screen-panel geometry.
- `track: LayoutTrack` - resolves the active `Scene` at any event time.
- `cam_moves: CameraMoveTrack` - built once from `doc.camera_moves` (Task 4); `step_camera` samples it per frame to override the scene's camera-panel rect. Empty (the default) means "no override" - see `## camera` below.
- `regions: Vec<ZoomRegion>` - zoom regions with RAW canvas-space anchors (`fromedit::regions_from_doc`). *Why raw:* `step_camera` re-anchors them into each frame's own screen panel (`layout::anchor_frame`), so a layout transition or display switch mid-zoom carries a pinned aim with the content instead of leaving the camera on where the panel used to be (2026-09-14).
- `frame_regions: Vec<ZoomRegion>` - the scratch buffer that per-frame re-anchoring fills, handed to `CameraSim::step`; kept on the struct so a 60 fps walk allocates nothing.
- `bg: Vec<u8>` - decoded background BGRA pixels; passed to compositor each frame.
- `compositor: Box<dyn Compositor>` - GPU or CPU compositor, selected once at init.
- `fx: Box<dyn FxRenderer>` - GPU or CPU FX renderer.
- `sim: CameraSim` - stateful; must be stepped in frame-loop order.
- `spot_sim: fx_state::SpotlightSim` - stateful spotlight animation phase (breathing/halo/etc.), stepped alongside `sim` each frame inside `fx_state::render`.
- `cursor: Cursor` - single owner of the mouse-event log and the cursor's polish settings - smoothness/idealize, applied by its rest/move path model (`cursor/path.md`) - (also the FX event source, via `cursor.events()`). Replaces an older split where `FrameRenderer` held the event log directly alongside inline `cur_idx`/`cur_sx`/`cur_sy`/`cur_primed` smoothing fields, because a borrowed `Cursor<'a>` couldn't self-reference the same struct's event log without unsafe; `Cursor` now owns its data outright; constructed once in `new()` via `Cursor::new(events, screen, smoothness)` plus `set_idealize(path_idealize)` and `set_tilt(tilt)` (both through their `_at` variants, so plain-OS mode gets the raw path and no lean), and `step_camera` simply calls `self.cursor.at(ev_t, dt_ms)` - passing its own exact frame period through, so the motion-tilt filter and the camera's filters agree on how long a frame is. *Why this matters for edits:* `reload_edit` can live-update all three in place (`set_smoothness`/`set_idealize`/`set_tilt`) when only `smoothness`/`path_idealize`/`tilt` changed, without rebuilding the renderer.
- `cprep: Option<CursorPrep>` - decoded cursor sprite set; `None` for non-Enhanced styles (except the plain-OS arrow fallback, see `cursorset::draws_synthetic`).
- `captured: Option<CapturedCursors>` - the recording's captured OS-cursor layer, decoded once (`export::cursor::captured`). `Some` means the REAL cursor can be composited, which is what the LIVE `System` style draws instead of the synthetic arrow; `None` is a pre-layer recording. Built in `new` beside `cprep`, for the same reason: it is edit-independent, so a warm preview must not redo it on every doc change.
- `actions: Vec<ActionEvent>` - action log; used by `fx_state::render` for spotlight/hold/caption.
- `effects: Vec<crate::edit::model::EffectRegion>` - the doc's effect regions (OUTPUT clock, like every `EditDoc` region list), refreshed by `reload_edit`; read by `fx_state::render` at `pose.out_t` while `actions` beside it is read at `pose.ev_t`.
- `captions: Vec<crate::edit::captions::Caption>` - the spoken-caption track, also on the OUTPUT clock and also refreshed by `reload_edit`. *Why only the track has a field:* its look and its two ASR inputs live on `settings.captions` (ADDED-4), and `settings` is already here, so a second field would be a second copy of the same thing.
- `has_webcam: bool` - whether `webcam.mp4` exists on disk, computed ONCE in `new` via `paths.webcam().exists()` (recording is already finished by render time, so this cannot change - `reload_edit` does not recompute it). Gates the spotlight's "keep camera lit" hole (`fx_state::fx_state_at`'s `has_hole` check) so a recording with no webcam, or a frame where the camera panel isn't visible, never draws an un-dimmed empty rectangle.
- `sw, sh: u32` - screen capture dimensions.
- `events_ms: u64` - wall-clock offset of event-time 0; `t - events_ms` is the EVENT clock, used for the raw streams (cursor position + sprite, click ripples, hold-driven video FX, captions).
- `video_start: u64` - capture timestamp of frame 0; `t - video_start` is the OUTPUT clock, used for everything that came out of `edit.json` (zoom regions, layout segments, effect regions, camera moves). *Why both are kept:* they differ by ~800 ms on a real recording, so a consumer sampled on the wrong one fires visibly early or late. `EditState::load` also gets `events_ms - video_start` from these two, for the one place a recorded (event-clock) track can still reach the renderer.

### Used by

`exporter::export` (the export loop), and Task 2 `preview.rs` (the preview engine).

`reload_edit`, `background`, `swap_bg` and the `BG_MESH` constant now live in the sibling `render/bg.rs` (this file was at its line budget when the background gained an asset path). They are still methods on this same `FrameRenderer` - see `docs/api/src-tauri/src/export/render/bg.md`.

## FrameRenderer::new

```rust
pub fn new(paths: &ProjectPaths, layout: Layout, fps: u32, resolution: Resolution, preview_cap: Option<u32>,
    system: &dyn SystemPort) -> Result<(Self, RenderMeta)>
```

Loads the edit doc and event log, resolves all per-export setup, and returns both the renderer and the metadata the caller needs to spawn decoders. The `edit.json`-derived state (settings, zoom config, layout track, anchored regions, effects) is built via `render_edit::EditState`, shared with `reload_edit` so a warm-preview edit can refresh it in place.

### Inputs (what, and why it is needed)

- `paths: &ProjectPaths` - root of the project folder. *Why:* all asset paths (`events.json`, `actions.json`, `video`, `webcam`, `cursor.json`, `sync.json`, `edit.json`) are derived from it.
- `layout: Layout` - output canvas geometry SEED. *Why:* taken by value; `new` resolves the doc's `aspect` (RATIO) and `resolution` (SIZE) against the true probed source dims via `layout.resolve(...)` before using it, so the caller's own `out_w`/`out_h` are overwritten regardless (the exporter passes `Layout::default()`; the preview engine also passes `Layout::default()` and relies on `preview_cap` to downscale).
- `fps: u32` - capture frame rate. *Why:* passed to `build_timeline` as a last-resort denominator when `sync.json` is absent and no audio duration is available; matches `exporter::export`'s own `capture_fps` (unrelated to the export's OUTPUT frame rate, which comes from `ExportSettings.fps` / `Fps::resolve_hz`).
- `resolution: Resolution` (`export::settings::Resolution`) - the user's chosen export SIZE preset (short-edge px), combined with `aspect`'s RATIO in `Layout::resolve`. *Why a separate param from `aspect`:* export-settings resolution and the doc's own aspect ratio are independent choices (see `ExportSettings`). Preview call sites (`preview::build_renderer`, and the `render_edit` tests) always pass `Resolution::Source` (a no-op) since the export resolution setting only applies to the export build.
- `preview_cap: Option<u32>` - `None` for a full export build (no downscale); `Some(long_edge)` downscales the aspect+resolution-resolved frame to that budget (`Layout::resolve`) for a cheap preview build, scaling `pad_px`/`screen_radius_px` proportionally.
- `system: &dyn SystemPort` - read exactly once, for `os_prefers_dark`, which `settings::theme::resolve_dark` needs to turn a `ThemeMode::System` into a colour decision. *Why a port and not a `Platform`:* this is the ONE OS fact the render depends on, and taking the single port says so - the render cannot reach for a capture or an input stream by accident. *Why it is a parameter at all:* until Batch D this was `crate::platform::resolve_dark(theme)`, a free function reaching into the registry from inside the renderer's constructor. That made the renderer untestable against a chosen theme and hid a platform dependency in the middle of the export, which is the one tree "export is the reference" says must not drift. The composition roots that supply it are `export::preview::with_warm_app` (from `Arc<Platform>` in app state), `export::pipeline::run::run_export`, and the `#[ignore]`d benches, which build their own bundle.

### Returns

`Result<(FrameRenderer, RenderMeta)>`. Errors on `events.json` load failure or `probe_dims` failure (video file missing or unreadable).

### Implementation

Follows the same sequence as the original `exporter::export` setup block (lines ~32-107), with the encoder/sink and the `RawDecoder::spawn` calls excluded. `build_timeline` runs BEFORE `EditState::load`, because `EditState` needs `events_ms - video_start` for its recorded-action layout fallback. Peeks `edit::seed::load_or_seed(paths)` once, up front, for `seed.aspect` (resolves `layout` together with the caller's `resolution`) and `seed.trim` (carried into `RenderMeta.trim` unresolved). The webcam decode box is computed here and returned as `RenderMeta.webcam_w`/`webcam_h` so the exporter does not need to repeat it: every layout mode's resolved overlay plus the webcam file's own probed dims go through `meta::webcam_box(&panels, wc_src, 1440)`, which gives ONE box at the SOURCE's aspect that each panel cover-crops at composite time. It replaced a box picked from the largest PANEL (`max_by_key((size_px, width_px))`): with a single panel-shaped box, every layout whose aspect disagreed with the winner was stretched (a square fed to a 16:9 bubble) or squashed (16:9 fed to the square big-cam) by 1.78x for its whole segment, and since the bubble and big-camera families never agree, one of them always lost. `has_webcam = paths.webcam().exists()` is also computed here (a plain filesystem check) and stored on `self` for `composite_at` to pass to `fx_state::render`; it also gates the one extra `probe_dims` subprocess (no webcam file, no probe).

## spans

The take's SOURCE SPANS and the display-switch transition - see `docs/api/src-tauri/src/export/render/spans.md`.

## screen_mix

The nv12 cross-dissolve a display switch runs through - see `docs/api/src-tauri/src/export/render/screen_mix.md`.

Runs the full compositor + FX + cursor pipeline for one frame, writing the result into `out`. Expensive (GPU or multi-core CPU work).

### Inputs (what, and why it is needed)

- `pose: &FramePose` - the resolved pose from `step_camera`. *Why:* the compositor, FX renderer, and cursor draw all need the same scene/camera/cursor values; passing one struct keeps the call site clean.
- `screen: &[u8]` - decoded screen frame as **NV12** (`sw*sh*3/2` bytes: Y plane + half-res interleaved UV), NOT BGRA. *Why:* the screen `RawDecoder` is spawned with pixel format `"nv12"` (cheaper to decode/pipe than BGRA); the compositor converts it to BGRA internally (GPU path: in-shader; CPU path: `color::nv12_to_bgra` up front) before blending it into the output canvas.
- `webcam: Option<(&[u8], u32, u32)>` - decoded webcam frame (bytes, width, height) or `None`. *Why:* the compositor uses this for the camera panel; `None` when webcam is absent or exhausted.
- `out: &mut Vec<u8>` - caller-owned output buffer. *Why:* lets the caller (exporter, preview) reuse the same allocation across frames instead of allocating a fresh `Vec` each call; threaded straight through to `compositor.composite_into`.

### Returns

Nothing (`()`). On return, `out` holds BGRA pixels (`out_w * out_h * 4` bytes) - resized and fully overwritten by the compositor, then mutated in place by the FX and cursor stages.

### Implementation

Three sequential stages (the original exporter loop body, lines ~138-144, plus the caption blit M5 T5 added between the FX pass and the cursor):
1. `compositor.composite_into(..., out)` - places screen + webcam into `out` with zoom crop and panel rounding.
2. `fx_pass(pose, out, ow, oh, lens)` (`render/fx_step.rs` - see `fx_step.md`) - the FX pass, then the caption overlay. The two calls below used to sit inline here; they moved out verbatim so the parity features (masks, grade, animated text) each add one line to one small file instead of three agents editing this function. Everything about the order and the clocks is unchanged:
   - `fx_state::render(..., &self.cursor.screen(), self.has_webcam, ..., pose.out_t, pose.ev_t, ...)` - applies click rings, spotlight, video FX, and the HOTKEY chord overlay (`hotkeycap`, not the spoken-caption track) directly on `out`. BOTH clocks are passed: the doc's effect regions resolve at `pose.out_t` (`region_t`), while the raw click/hold/chord streams resolve at `pose.ev_t`. `self.cursor.screen()` supplies the capture origin so a click hit's raw desktop coordinates convert to screen-local the same way `Cursor`'s own `clicks`/`at` already do (`FrameRenderer` has no separate `ScreenInfo` field - it reads the one `Cursor` already owns). `self.has_webcam` gates the spotlight's camera-exclusion hole (see `FrameRenderer`'s field list above).
   - `captiondraw::overlay(out, ow, oh, &self.captions, &self.settings.captions, self.settings.ui.accent, pose.out_t)` - the spoken captions, on the OUTPUT clock like every doc region list. It sits HERE, after the FX pass and before the cursor, because a caption belongs over the picture and its effects but never over the pointer. It is called directly rather than through `fx_state::render`, which already carries eighteen arguments and a `#[allow(clippy::too_many_arguments)]`. Because every format composites through `composite_at`, GIF export keeps captions with no extra path; `Format::Gif`'s no-audio mux is untouched.
3. The cursor, on `ev_t` (both cursor tracks are raw event streams, so they stay on the event clock). ONE of two paths runs, never both: `captured::draws_captured(self.settings.cursor.style, self.captured.is_some())` - live style is `System` and this recording has a layer - blits the REAL recorded bitmap via `CapturedCursors::draw`; otherwise `cursorset::draw` (if `cprep.is_some()`) blits the synthetic sprite with motion trail, bounce, and panel clipping. That one ALSO gets `pose.out_t`, the clock pack v2's animated busy cursor runs on - the only cursor input on the output clock rather than the event clock, so the animation is deterministic per rendered frame. That single gate is what confines the plain-OS arrow fallback to recordings with no layer to composite. Both take the same `(cur, cam, screen, inset_w, ev_t)` - the captured one additionally takes `self.sw`, the source width its `content_scale` needs - and project through the shared `cursorset::frame_placement`, so a style switch cannot move the cursor.

The synthetic path also gets `self.cursor.tilt_deg()`, the motion lean the `Cursor` computed on the same frame `step_camera` asked it for a position, and `FrameRenderer::lenses` puts the identical value into `LensFrame.tilt_deg` - so the glass lens placed BEFORE the FX pass and the sprite blitted AFTER it lean by the same angle. The captured path is deliberately left out: that is the cursor that was actually on screen, and it never leaned.

`FrameRenderer`'s small read-only accessors used only by preview commands outside the renderer (`bg`, `has_webcam`, `click_track`, `events_ms`, `actions`, `resolve_layout`) live in the sibling `accessors.rs` (split out purely for size) - see `accessors.md`.

## fromedit

Reconstructs `ZoomRegion` and `SetLayout` action tracks from a persisted `EditDoc`, the inverse of `edit::seed`. Key items: `regions_from_doc(doc, sw, sh) -> Vec<ZoomRegion>`, `layout_segs_from_doc(doc) -> Option<Vec<ActionEvent>>`.

## step

The per-frame camera step (`reset_camera`, `snap_cursor`, `step_camera` on the two clocks) and `walk_plan`, the one frame-plan walk the exporter, the one-shot preview and `camera_track` share. Moved out of this file at the size cap; see `step.md`.

## composite

The per-frame composite pass (`composite_at`) and its lens build - see `docs/api/src-tauri/src/export/render/composite.md`.
