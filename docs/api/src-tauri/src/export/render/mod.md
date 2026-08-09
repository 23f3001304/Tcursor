# src-tauri/src/export/render/mod.rs

Reusable per-frame renderer that owns all per-export setup except the raw decoders and the encoder sink. Extracted from `exporter.rs` so the same compositing code can serve both the export loop and the preview engine (Task 2). The exporter calls `FrameRenderer::new` once, then `step_camera` + `composite_at` once per output frame. The preview engine fast-forwards `step_camera` from 0 to a target time (cheap math, no decode) and composites once.

## OUT_FPS

```rust
pub const OUT_FPS: u64 = 60
```

Constant output frame rate (60 fps). Defined here so both `exporter.rs` and future preview code share a single source of truth for the encode rate.

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
- `regions: Vec<ZoomRegion>` - re-anchored zoom regions for `CameraSim::step`.
- `bg: Vec<u8>` - decoded background BGRA pixels; passed to compositor each frame.
- `compositor: Box<dyn Compositor>` - GPU or CPU compositor, selected once at init.
- `fx: Box<dyn FxRenderer>` - GPU or CPU FX renderer.
- `sim: CameraSim` - stateful; must be stepped in frame-loop order.
- `spot_sim: fx_state::SpotlightSim` - stateful spotlight animation phase (breathing/halo/etc.), stepped alongside `sim` each frame inside `fx_state::render`.
- `cursor: Cursor` - single owner of the mouse-event log and the cursor low-pass/idealize state (also the FX event source, via `cursor.events()`). Replaces an older split where `FrameRenderer` held the event log directly alongside inline `cur_idx`/`cur_sx`/`cur_sy`/`cur_primed` smoothing fields, because a borrowed `Cursor<'a>` couldn't self-reference the same struct's event log without unsafe; `Cursor` now owns its data outright; constructed once in `new()` via `Cursor::new(events, screen, follow_alpha)` plus `set_idealize(path_idealize)`, and `step_camera` simply calls `self.cursor.at(ev_t)`. *Why this matters for edits:* `reload_edit` can live-update the low-pass alpha and idealize amount in place (`set_a`/`set_idealize`) when only `smoothness`/`path_idealize` changed, without rebuilding the renderer.
- `cprep: Option<CursorPrep>` - decoded cursor sprite set; `None` for non-Enhanced styles.
- `actions: Vec<ActionEvent>` - action log; used by `fx_state::render` for spotlight/hold/caption.
- `effects: Vec<crate::edit::model::EffectRegion>` - the doc's effect regions (OUTPUT clock, like every `EditDoc` region list), refreshed by `reload_edit`; read by `fx_state::render` at `pose.out_t` while `actions` beside it is read at `pose.ev_t`.
- `has_webcam: bool` - whether `webcam.mp4` exists on disk, computed ONCE in `new` via `paths.webcam().exists()` (recording is already finished by render time, so this cannot change - `reload_edit` does not recompute it). Gates the spotlight's "keep camera lit" hole (`fx_state::fx_state_at`'s `has_hole` check) so a recording with no webcam, or a frame where the camera panel isn't visible, never draws an un-dimmed empty rectangle.
- `sw, sh: u32` - screen capture dimensions.
- `events_ms: u64` - wall-clock offset of event-time 0; `t - events_ms` is the EVENT clock, used for the raw streams (cursor position + sprite, click ripples, hold-driven video FX, captions).
- `video_start: u64` - capture timestamp of frame 0; `t - video_start` is the OUTPUT clock, used for everything that came out of `edit.json` (zoom regions, layout segments, effect regions, camera moves). *Why both are kept:* they differ by ~800 ms on a real recording, so a consumer sampled on the wrong one fires visibly early or late. `EditState::load` also gets `events_ms - video_start` from these two, for the one place a recorded (event-clock) track can still reach the renderer.

### Used by

`exporter::export` (the export loop), and Task 2 `preview.rs` (the preview engine).

## FrameRenderer::reload_edit

```rust
pub fn reload_edit(&mut self, paths: &ProjectPaths)
```

Refreshes the `edit.json`-derived state (settings, zoom config, layout track, anchored regions, effects) in place, via `render_edit::EditState`. Called by `preview::with_warm` when only `edit.json` changed: it avoids a full `new()` - no new GPU device, no video probe, no cursor prep - so an editor zoom/spotlight edit costs ~microseconds instead of rebuilding the renderer (~seconds).

Two cursor-motion settings ARE live-applied here, before `self.settings` is overwritten: `self.cursor.set_a(es.settings.cursor.follow_alpha())` (a `Smoothness` slider change) and `self.cursor.set_idealize(es.settings.cursor.path_idealize)` (the path-straightening amount) - both just update in-place state on the existing `Cursor`, no decode or GPU work, so these two settings take effect on the very next preview frame. Cursor prep (`cprep`, the decoded sprite set) is still NOT refreshed: it is edit-independent for the metadata the live editor uses (only `composite_at` reads it, which the editor's canvas preview never calls), so any OTHER cursor setting (style, pack, size, motion blur, click bounce) still needs a full rebuild to reach `composite_at`.

**Background is the one exception to "no background decode":** `bg` IS conditionally rebuilt here, via `background::build`, but ONLY when `es.settings.background != self.settings.background` (compared before `self.settings` is overwritten). This keeps the common case (a zoom/cursor/effects edit, background unchanged) exactly as cheap as before, while a background-panel edit (type/color/gradient/blur) still reaches the warm preview renderer instead of requiring a full rebuild. The comparison matters because `BackgroundKind::Mesh` decodes via an ffmpeg subprocess - unconditionally rebuilding on every `reload_edit` call would make unrelated edits noticeably slower.

## FrameRenderer::new

```rust
pub fn new(paths: &ProjectPaths, layout: Layout, fps: u32, resolution: Resolution, preview_cap: Option<u32>) -> Result<(Self, RenderMeta)>
```

Loads the edit doc and event log, resolves all per-export setup, and returns both the renderer and the metadata the caller needs to spawn decoders. The `edit.json`-derived state (settings, zoom config, layout track, anchored regions, effects) is built via `render_edit::EditState`, shared with `reload_edit` so a warm-preview edit can refresh it in place.

### Inputs (what, and why it is needed)

- `paths: &ProjectPaths` - root of the project folder. *Why:* all asset paths (`events.json`, `actions.json`, `video`, `webcam`, `cursor.json`, `sync.json`, `edit.json`) are derived from it.
- `layout: Layout` - output canvas geometry SEED. *Why:* taken by value; `new` resolves the doc's `aspect` (RATIO) and `resolution` (SIZE) against the true probed source dims via `layout.resolve(...)` before using it, so the caller's own `out_w`/`out_h` are overwritten regardless (the exporter passes `Layout::default()`; the preview engine also passes `Layout::default()` and relies on `preview_cap` to downscale).
- `fps: u32` - capture frame rate. *Why:* passed to `build_timeline` as a last-resort denominator when `sync.json` is absent and no audio duration is available; matches `exporter::export`'s own `capture_fps` (unrelated to the export's OUTPUT frame rate, which comes from `ExportSettings.fps` / `Fps::resolve_hz`).
- `resolution: Resolution` (`export::settings::Resolution`) - the user's chosen export SIZE preset (short-edge px), combined with `aspect`'s RATIO in `Layout::resolve`. *Why a separate param from `aspect`:* export-settings resolution and the doc's own aspect ratio are independent choices (see `ExportSettings`). Preview call sites (`preview::build_renderer`, and the `render_edit` tests) always pass `Resolution::Source` (a no-op) since the export resolution setting only applies to the export build.
- `preview_cap: Option<u32>` - `None` for a full export build (no downscale); `Some(long_edge)` downscales the aspect+resolution-resolved frame to that budget (`Layout::resolve`) for a cheap preview build, scaling `pad_px`/`screen_radius_px` proportionally.

### Returns

`Result<(FrameRenderer, RenderMeta)>`. Errors on `events.json` load failure or `probe_dims` failure (video file missing or unreadable).

### Implementation

Follows the same sequence as the original `exporter::export` setup block (lines ~32-107), with the encoder/sink and the `RawDecoder::spawn` calls excluded. `build_timeline` runs BEFORE `EditState::load`, because `EditState` needs `events_ms - video_start` for its recorded-action layout fallback. Peeks `edit::seed::load_or_seed(paths)` once, up front, for `seed.aspect` (resolves `layout` together with the caller's `resolution`) and `seed.trim` (carried into `RenderMeta.trim` unresolved). The webcam decode box is computed here and returned as `RenderMeta.webcam_w`/`webcam_h` so the exporter does not need to repeat it: the LARGEST camera panel across all layout modes (`max_by_key((size_px, width_px))`, so a size tie prefers the wider one) run through `meta::webcam_dims(&ov, 1440)`, which keeps that panel's own aspect instead of forcing a square - a `CamAspect::Wide` panel is 16:9 and a square decode was being stretched 1.78x across it by the compositor. `has_webcam = paths.webcam().exists()` is also computed here (a plain filesystem check, not a probe) and stored on `self` for `composite_at` to pass to `fx_state::render`.

## FrameRenderer::step_camera

```rust
pub fn step_camera(&mut self, t: u64) -> FramePose
```

Advances the camera simulation to output time `t` and returns the resolved pose. Cheap: only arithmetic, no I/O. Must be called in ascending `t` order because `CameraSim` and the cursor index are stateful.

### Inputs (what, and why it is needed)

- `t: u64` - capture wall-clock time in ms (same epoch as `RenderMeta.video_start`). *Why:* both clocks are derived from this one absolute time - `ev_t = t - events_ms` for the raw event streams, `out_t = t - video_start` for everything stored in `edit.json`.

### Returns

`FramePose` with both clocks (`ev_t`, `out_t`), `scene` (possibly camera-shrunk), `cur` (panel-mapped cursor), and `cam` (damped camera).

### Implementation

Calls `self.cursor.at(ev_t)` for the smoothed cursor position - `Cursor` now owns its event log outright (built once in `FrameRenderer::new` via `Cursor::new(events, screen, follow_alpha)`), so `step_camera` just delegates instead of replicating the index-advance + exponential low-pass inline the way an earlier version had to when `FrameRenderer` held `cur_idx`/`cur_sx`/`cur_sy`/`cur_primed` alongside a borrowed `Cursor<'a>`. The low-pass alpha is no longer a hardcoded constant: it comes from `CursorSettings::follow_alpha()` (derived from the `smoothness` setting; default `0.6` -> alpha `~0.36`, matching the old hardcoded `0.35`) and can change live via `Cursor::set_a` in `reload_edit` without rebuilding the renderer. The cursor samples at `ev_t = t - events_ms` (it indexes the raw mouse log); `LayoutTrack::scene_at` and `CameraSim::step` both sample at `out_t = t - video_start`, because the layout segments and zoom regions they hold both come from `edit.json` and are therefore output-time. (`LayoutTrack::scene_at` was sampled at `ev_t` before the one-clock fix, which put every layout switch ~800 ms early in exports; the recorded-action fallback track is now shifted to output time when it is built, in `EditState::load`.) After the camera step come the CameraOnly identity override (`scene.screen.alpha < 0.5`) and then the webcam-on-zoom action (`cam_action_at` + `apply_cam_zoom_action`) when the screen panel is dominant. `cam_action_at` returns `(action, target_scale)` - the WINNING REGION'S OWN `target_scale`, not `self.cfg.target_scale` (the global default) - so `apply_cam_zoom_action`'s `zoom_progress` reaches 1.0 at the region's own peak zoom, not the global one. `self.cfg.target_scale` still feeds `self.sim.step` above (the camera framing itself, a separate concern from the webcam-shrink action). **Keyframes win while they own the frame:** the action is skipped entirely on any frame where `CameraMoveTrack::sample` returned a pose, because the override already decides the PiP there - previously the shrink ran on top of an override, silently scaling a hand-keyframed camera during zooms. Since Task 27 that is a per-frame question rather than a whole-clip one: outside the keyframes' span `sample` is `None`, so the smart shrink applies normally again. The legacy `camera_shrink` bool is no longer checked here; it is folded into `ZoomSettings::resolved_cam_action` (toggle off resolves to `Stay`, the identity).

**`camera_moves` override (Task 4; radius/ring fix in Task 9 Part C; span semantics in Task 27):** right after `scene` is resolved and `out_t` is computed, `step_camera` derives the LIVE layout-resolved pose from the just-resolved `scene.camera.rect` and samples `self.cam_moves` (a `CameraMoveTrack` built once from `doc.camera_moves` in `EditState::load`, refreshed by both `FrameRenderer::new` and `reload_edit`) with it:

```rust
let (ow, oh) = (self.layout.out_w as f32, self.layout.out_h as f32);
let live = Some(static_cam_pose(scene.camera.rect, ow, oh));
let cam_aspect = scene.camera.rect.w / scene.camera.rect.h.max(0.001);
if let Some(p) = self.cam_moves.sample(out_t, live) {
    scene.camera = crate::export::scene::override_camera(scene.camera, p, ow, oh, cam_aspect);
}
```

- `static_cam_pose(rect, ow, oh) -> CamPose` (`export/camera/mod.rs`) - the inverse of `rect_from_center`: converts the RESOLVED (un-overridden) camera panel's rect into a `CamPose` (center x/y + height fraction), the "what the webcam would show with zero `camera_moves`" pose. Recomputed EVERY frame from that frame's own scene, which is what makes it a *live* pose rather than a static one: if a `LayoutTrack` cross-fade is moving the panel, this moves with it.
- `CameraMoveTrack::sample(out_t, live) -> Option<CamPose>` (`export/camera/moves.rs`) - `None` for an empty track, which is the seeded-doc default, so this block never runs and the scene's camera panel is exactly whatever `overlay_for`/`resolve` produced (byte-identical to pre-Task-4 behavior). **Task 27:** `None` ALSO whenever `out_t` falls outside `[first - KF_BLEND_MS, last + KF_BLEND_MS]`, so keyframes override only their own span and layout segments own the panel everywhere else - before this, one keyframe anywhere made `sample` return `Some` for the entire clip and silently stomped every layout segment. Inside the span the track eases FROM `live` into the first keyframe over the entry window, interpolates keyframe-to-keyframe (unchanged math), and eases from the last keyframe BACK to `live` over the exit window - and because `live` is this frame's value, that exit blend tracks a layout transition that is still moving, the same principle as `CameraSim`'s driver handoff.
- `cam_aspect` - the STATIC panel's own width/height, read off `scene.camera.rect` in the same breath as `live` (i.e. before the override replaces it) and passed through `override_camera` into `rect_from_center`. *Why derived from the rect rather than re-read from `appearance.cam_aspect`:* `resolve`/`bubble_rect` already turned the setting into `width_px`/`size_px` for whichever preset (or `LayoutTrack` cross-fade between presets) is live this frame, so the rect is the resolved truth and can never disagree with the panel being overridden. Without it a single `camera_moves` keyframe squared a Wide (16:9) panel for the rest of the clip, because the pose only carries height.
- `override_camera(panel: Panel, p: CamPose, ow: f32, oh: f32, aspect: f32) -> Panel` (`export/scene/mod.rs`) - replaces `scene.camera` wholesale (not just its rect): the new rect comes from `rect_from_center` (height `h = p.size * oh`, width `w = h * aspect`), and `radius`/`ring_px` are scaled by the height ratio `new_h / old_h.max(0.001)` so a circle panel (`radius == min(w,h)/2` at its static size) stays a true circle instead of distorting toward the pre-override radius - the bug this Task 9 Part C fix corrects. `alpha`/`ring_color` are carried over unchanged.
- The override runs before the `camera_shrink` block below it, so an in-flight zoom-shrink composes on top of the overridden panel's center/radius, same as it would on top of the static one.
- Uses the same `ow`/`oh` (`self.layout.out_w`/`out_h`, `f32`) already in scope for the surrounding per-frame math - no separate output-dimension lookup.

## FrameRenderer::composite_at

```rust
pub fn composite_at(&mut self, pose: &FramePose, screen: &[u8],
                    webcam: Option<(&[u8], u32, u32)>, out: &mut Vec<u8>)
```

Runs the full compositor + FX + cursor pipeline for one frame, writing the result into `out`. Expensive (GPU or multi-core CPU work).

### Inputs (what, and why it is needed)

- `pose: &FramePose` - the resolved pose from `step_camera`. *Why:* the compositor, FX renderer, and cursor draw all need the same scene/camera/cursor values; passing one struct keeps the call site clean.
- `screen: &[u8]` - decoded screen frame as **NV12** (`sw*sh*3/2` bytes: Y plane + half-res interleaved UV), NOT BGRA. *Why:* the screen `RawDecoder` is spawned with pixel format `"nv12"` (cheaper to decode/pipe than BGRA); the compositor converts it to BGRA internally (GPU path: in-shader; CPU path: `color::nv12_to_bgra` up front) before blending it into the output canvas.
- `webcam: Option<(&[u8], u32, u32)>` - decoded webcam frame (bytes, width, height) or `None`. *Why:* the compositor uses this for the camera panel; `None` when webcam is absent or exhausted.
- `out: &mut Vec<u8>` - caller-owned output buffer. *Why:* lets the caller (exporter, preview) reuse the same allocation across frames instead of allocating a fresh `Vec` each call; threaded straight through to `compositor.composite_into`.

### Returns

Nothing (`()`). On return, `out` holds BGRA pixels (`out_w * out_h * 4` bytes) - resized and fully overwritten by the compositor, then mutated in place by the FX and cursor stages.

### Implementation

Three sequential stages (identical to the original exporter loop body, lines ~138-144):
1. `compositor.composite_into(..., out)` - places screen + webcam into `out` with zoom crop and panel rounding.
2. `fx_state::render(..., &self.cursor.screen(), self.has_webcam, ..., pose.out_t, pose.ev_t, ...)` - applies click rings, spotlight, video FX, and captions directly on `out`. BOTH clocks are passed: the doc's effect regions resolve at `pose.out_t` (`region_t`), while the raw click/hold/caption streams resolve at `pose.ev_t`. `self.cursor.screen()` supplies the capture origin so a click hit's raw desktop coordinates convert to screen-local the same way `Cursor`'s own `clicks`/`at` already do (`FrameRenderer` has no separate `ScreenInfo` field - it reads the one `Cursor` already owns). `self.has_webcam` gates the spotlight's camera-exclusion hole (see `FrameRenderer`'s field list above).
3. `cursorset::draw(..., pose.ev_t, ...)` (if `cprep.is_some()`) - the cursor track is a raw event stream, so it stays on `ev_t`. Blits the Enhanced cursor sprite with motion trail, bounce, and panel clipping, directly on `out`.

`FrameRenderer`'s small read-only accessors used only by preview commands outside the renderer (`bg`, `has_webcam`, `click_track`, `events_ms`, `actions`, `resolve_layout`) live in the sibling `accessors.rs` (split out purely for size) - see `accessors.md`.

## fromedit

Reconstructs `ZoomRegion` and `SetLayout` action tracks from a persisted `EditDoc`, the inverse of `edit::seed`. Key items: `regions_from_doc(doc, sw, sh) -> Vec<ZoomRegion>`, `layout_segs_from_doc(doc) -> Option<Vec<ActionEvent>>`.
