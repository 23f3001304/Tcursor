# src-tauri/src/export/render/meta.rs

Small data-only types returned by `FrameRenderer`, split out of `mod.rs` (which was at the 200-line budget) purely for size. `RenderMeta`/`FramePose` are re-exported as `render::RenderMeta`/`render::FramePose` (`pub use meta::{FramePose, RenderMeta};` in `mod.rs`) so existing callers are unaffected by the split; `webcam_dims` (the one small function here) sits with the field it fills in.

## RenderMeta

```rust
pub struct RenderMeta {
    pub tl: Timeline, pub video_start: u64, pub video_end: u64, pub out_w: u32, pub out_h: u32,
    pub sw: u32, pub sh: u32, pub screen_bytes: usize, pub audio_offset_ms: i32,
    pub webcam_w: u32, pub webcam_h: u32,
    pub trim: crate::edit::model::Trim,
    pub mic_volume: f32, pub sys_volume: f32,
}
```

All information the export (or preview) loop needs to set up its raw decoders and drive the frame loop, returned by `FrameRenderer::new` so the caller never needs to re-read the project files.

**Fields and why each is here:**

- `tl: Timeline` - per-frame wall-clock timestamps and audio offsets from `build_timeline`. *Why:* the export loop indexes this to advance the screen decoder to the captured frame active at each output time.
- `video_start: u64` - timestamp (ms) of the first captured frame. *Why:* the loop's time variable `t = video_start + k * 1000 / OUT_FPS`.
- `video_end: u64` - timestamp of the last captured frame (at least `video_start + 1`). *Why:* `total_out = (video_end - video_start) * OUT_FPS / 1000` drives the loop bound.
- `out_w: u32`, `out_h: u32` - output canvas dimensions in pixels. *Why:* `FrameRenderer` owns the layout after `new()`, so the caller gets these from meta rather than re-reading the layout struct.
- `sw: u32`, `sh: u32` - raw screen capture dimensions (probe result). *Why:* needed to allocate the `screen_bytes`-sized decode buffer.
- `screen_bytes: usize` - `sw * sh * 3 / 2`: exact NV12 buffer size for the screen decoder (Y plane + half-res interleaved UV - cheaper to decode/pipe than BGRA; the compositor converts to BGRA internally). *Why:* pre-computed to avoid re-doing the multiply at every `RawDecoder::spawn` call.
- `webcam_w: u32`, `webcam_h: u32` - the webcam decode box in pixels (`webcam_dims` below), height capped to 1440. *Why a pair and not one square side:* the decoder cover-crops the webcam to exactly these dims and the compositor then stretches that buffer across the camera panel, so the two must share an aspect - a `CamAspect::Wide` panel is 16:9 (`width_px = size_px * 16/9`) and a square decode was being stretched 1.78x across it. `RawDecoder::spawn` takes the pair as its `cover_scale` argument; `exporter`/`preview` allocate `webcam_w * webcam_h * 4` to match.
- `audio_offset_ms: i32` - the user's manual mic-sync nudge from settings. *Why:* carried here so `exporter.rs` does not need to reload the edit doc a second time after `new()`.
- `trim: crate::edit::model::Trim` - the doc's trim window, unresolved (see `Trim::resolve`). *Why:* `exporter::export` resolves it against its own `video_end - video_start` and gates the frame loop (`pipeline::trim_frame_bounds`), again avoiding a second doc load.
- `mic_volume: f32`, `sys_volume: f32` - linear gain multipliers from `Settings.audio_mic_volume`/`audio_sys_volume` (0 = muted, 1 = unchanged, up to 1.5). *Why carried here rather than reloaded in `exporter.rs`:* same reasoning as `audio_offset_ms` - `FrameRenderer::new` already loaded settings once; `exporter::export` passes both straight through to `audio_mux::mux`.

### Used by

`exporter::export` reads every field to set up the decoders and drive `0..=total_out`.

## FramePose

```rust
pub struct FramePose { pub ev_t: u32, pub out_t: u32, pub scene: Scene, pub cur: FramePoint, pub cam: Camera }
```

The resolved camera and scene for one output frame, returned by `step_camera` and passed unchanged to `composite_at`. Grouping these values as a struct avoids passing them as separate arguments and lets the preview engine inspect the pose (e.g. to know zoom scale) without compositing.

**Fields and why each is here:**

- `ev_t: u32` - EVENT time in ms (`t - events_ms`). *Why:* the raw recorded streams - mouse events (click ripples), the action log (hold-driven video FX, captions) and the cursor track/sprite - are all timestamped on the input trackers' clock. Pre-computing it in `step_camera` avoids re-doing the subtraction in `composite_at`.
- `out_t: u32` - OUTPUT time in ms (`t - video_start`, 0 = first video frame). *Why:* every `EditDoc` region list (zooms, layout, effects, camera_moves) is stored on this clock, and it is the clock the editor timeline and TS preview use. `composite_at` needs it as well as `ev_t` because the FX stage samples effect REGIONS on one clock and raw streams on the other. The two differ by ~800 ms on a real recording.
- `scene: Scene` - the two-panel layout (screen + camera rects/alphas) possibly modified by camera-shrink. *Why:* the compositor, FX renderer, and cursor draw all need the scene; carrying it in the pose avoids re-calling `track.scene_at`.
- `cur: FramePoint` - cursor position mapped into the screen panel (output pixels). *Why:* needed by the compositor for the zoom setpoint, by FX for click-ring placement, and by the cursor sprite draw.
- `cam: Camera` - zoom center + scale for this frame. *Why:* the compositor crops and resizes to this, and the FX + cursor layers project through it.

### Used by

`composite_at` reads all four fields. The preview engine (Task 2) will inspect `cam` and `scene` to derive the zoom level for display.

## webcam_dims

```rust
pub fn webcam_dims(ov: &crate::export::types::OverlayLayout, cap: u32) -> (u32, u32)
```

The webcam decode box for one resolved camera-panel overlay: its own aspect, capped in height, rounded to even dims.

### Inputs

- `ov: &OverlayLayout` - the resolved overlay (`settings::appearance::overlay_for`) of the LARGEST camera panel across all layout presets. *Why the largest:* one decode box serves the whole export, so it must have enough pixels for the biggest panel any preset resolves to; `FrameRenderer::new` picks it with `max_by_key((size_px, width_px))`, so a size tie prefers the wider (Wide) panel and never under-samples.
- `cap: u32` - maximum decode HEIGHT in pixels (1440 in practice). *Why height and not area:* `size_px` is the height-driven knob (`cam_size * oh`) that every preset shares; width re-derives from the aspect afterwards, so capping can never skew the box.

### Returns

`(w, h)` where `h = size_px` clamped to `[2, cap]` and `w = round(h * width_px / size_px)`, both rounded DOWN to even numbers (min 2). *Why even:* keeps the dims safe for any decoder/filter that assumes chroma-aligned sizes. `CamAspect::Square` (`width_px == size_px`) reproduces the old square exactly.

### Behaviors worth knowing

- `wide_panel_decodes_at_sixteen_by_nine` (unit test): a Wide panel `252px` tall yields `(448, 252)` - the same `scale=448:252` box `decode_args` emits, not `252x252`.
- `square_panel_stays_square_and_even` (unit test): a Square panel is unchanged, and an odd `269` height rounds down to `268`.
- `cap_bounds_height_without_skewing_the_aspect` (unit test): a Wide panel taller than `cap` clamps to `cap` in height and still comes out 16:9, because width is derived AFTER the clamp.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::new` fills `RenderMeta.webcam_w`/`webcam_h` with it.
