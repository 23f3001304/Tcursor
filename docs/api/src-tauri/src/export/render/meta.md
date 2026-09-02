# src-tauri/src/export/render/meta.rs

Small data-only types returned by `FrameRenderer`, split out of `mod.rs` (which was at the 200-line budget) purely for size. `RenderMeta`/`FramePose` are re-exported as `render::RenderMeta`/`render::FramePose` (`pub use meta::{FramePose, RenderMeta};` in `mod.rs`) so existing callers are unaffected by the split; `webcam_box` (the one small function here) sits with the field it fills in.

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
- `webcam_w: u32`, `webcam_h: u32` - the webcam decode box in pixels (`webcam_box` below): the SOURCE video's own aspect, big enough for the largest camera panel, height capped to 1440 and to the source's own height. *Why the source's aspect and not a panel's:* one decode box serves the whole export, but the layout track can put a `CamAspect::Wide` (16:9) bubble and a square big-cam in the SAME export, so no panel-shaped box is right for all of them - whichever panel lost was stretched (or squashed) 1.78x for its whole segment. The compositors now cover-crop this box to each panel's aspect per frame (`gpu::compositor::cover_rect` / `shader.wgsl`'s `cover_uv`), which only works if the box still holds the full source frame. `RawDecoder::spawn` takes the pair as its `cover_scale` argument; `exporter`/`preview` allocate `webcam_w * webcam_h * 4` to match.
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

## webcam_box

```rust
pub fn webcam_box(panels: &[crate::export::types::OverlayLayout], src: Option<(u32, u32)>, cap: u32) -> (u32, u32)
```

The ONE webcam decode box for a whole export/preview: the source video's own aspect, sized to cover the largest camera panel on both axes.

### Inputs

- `panels: &[OverlayLayout]` - every layout preset's resolved overlay (`settings::appearance::overlay_for` for all five `LayoutId`s). *Why all of them and not the biggest:* the box has to cover whichever panel the layout track happens to be showing at any moment, so it is sized against the maximum over the whole set - `width_px.max(size_px)` for the needed WIDTH (bubble modes draw `width_px` wide, the big-camera modes a `size_px` square) and `size_px` for the needed HEIGHT.
- `src: Option<(u32, u32)>` - the webcam file's own pixel dims (`ffio::probe_dims`), or `None` when there is no webcam / it could not be probed. *Why the source shape drives the box:* every panel cover-crops this box at composite time, and a crop can only ever remove pixels - decoding a square (the old behaviour) has already thrown the sides of a 16:9 webcam away, so a Wide panel could then only get a zoomed-in band instead of the full frame. `None` falls back to a square box (the historical shape).
- `cap: u32` - maximum decode HEIGHT in pixels (1440 in practice). *Why a height cap:* `size_px` is the height-driven knob (`cam_size * oh`) every preset shares; width re-derives from the source aspect afterwards, so capping can never skew the box.

### Returns

`(w, h)`: `h = max(needed_h, ceil(needed_w / src_aspect))` clamped to `cap` AND to the source's own height, `w = round(h * src_aspect)`, both rounded DOWN to even numbers (min 2). *Why clamp to the source height:* decoding above the source resolution only makes ffmpeg upscale with `fast_bilinear` where the compositor (Lanczos3 on CPU, the sampler on GPU) does it at least as well - and it keeps the per-frame webcam byte cost at or below what the old square box moved.

### Behaviors worth knowing

- `the_box_takes_the_sources_aspect_not_a_panels` (unit test): a 1280x720 webcam decodes as the whole 1280x720 frame whether the bubble panels are Square or Wide; a 640x480 webcam stays 4:3.
- `the_box_is_clamped_by_the_source_and_the_cap` (unit test): a 4K export's 1920px-tall big-cam panel still decodes at most `cap` (1440) tall, and a 320x240 webcam is never upscaled by the decoder.
- `the_box_covers_the_widest_panel_on_a_tall_source` (unit test): a 9:16 portrait webcam grows in HEIGHT until the box is wide enough for the widest panel, and the source aspect survives that growth.
- `no_source_falls_back_to_a_square_box` (unit test): `None` gives a square, even-dimensioned box; an empty panel list gives the historical `(420, 420)`.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::new` fills `RenderMeta.webcam_w`/`webcam_h` with it (probing the webcam only when the file exists).
