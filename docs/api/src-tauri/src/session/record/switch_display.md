# src-tauri/src/session/record/switch_display.rs

Changing the recorded display (or window) while a take runs, from the HUD's Sources sheet (2026-09-14, `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`).

Two things move together and the take survives only if both do.

- **The pixels** move by restarting the WGC capture into the SAME encoder (`gpu_restart.rs`), so `video.mp4` stays one stream at the first display's size and every later frame is fitted into that canvas by `frame_scaler` / `frame_fit::letterbox`.
- **The input streams** move by remapping their coordinates at the hook (`events::remap`), because `events.json` carries one `ScreenInfo` and the export subtracts that one origin. Without the remap a click on the second monitor would be drawn on the first monitor's picture, a whole screen width away.

Nothing about the switch reaches the editor or the export: `sync.json`'s `display_switches` is for the record only, and `preprocess` has nothing to merge because a display switch produces no segment file.

The switch is also RECORDED, not just performed: each entry carries the new capture's `(w, h)` so the render can crop the fitted picture out of the canvas (`export::render::spans`).

## switch_display

```rust
#[tauri::command]
pub fn switch_display(target_id: String, recorder: State<'_, Recorder>, app: AppHandle) -> Result<(), String>
```

Switches the running take to `target_id` (`window:0x…` or `display:N`, the same ids `commands::list_displays` hands the HUD). `Err("not recording")` when no take is running; `Err("switching needs the GPU encoder; turn the compatibility encoder off")` when the take is on the legacy ffmpeg pipeline, whose rawvideo pipe is sized once at start and cannot be restarted into.

Everything happens under the recorder lock, so a switch cannot interleave with Pause, Resume or Stop.

### Implementation

1. Lock `Recorder::inner` and take `&mut Running`.
2. Build a `VideoStart` from the take's own `clock` / `stop` / `paused` / `paused_totals`, a fresh `record-ended-early` emitter, `fps` from `primary_refresh_hz().min(60)` and `with_cursor: false`. *Why fps is recomputed rather than remembered:* the encoder is already built, so this value only sets the new capture's `MinimumUpdateIntervalSettings` floor. *Why the cursor is still off:* it is captured as its own layer (`CursorTypeTracker`), which is what keeps every cursor style switchable in the editor.
3. `at_ms` from `recording_ms` (the pause-compressed recording clock, the same one the frame timestamps and the mic segments are on; see "The switch instant" below), then `target_bounds::get_target_bounds(Some(&target_id))` for the target's `(w, h, origin_x, origin_y)` - a monitor's rectangle, or a window's VISIBLE frame, which is what WGC captures.
4. Push `DisplaySwitch { at_ms, target_id, w, h }` into `Running.segments.displays` (written into `sync.json` at Stop by `recorder_threads::save_session_files`) BEFORE the restart, and build the `gpu_frames::SizeHook` that corrects it: a closure over the shared log calling `SegmentLog::set_display_size(at_ms, fw, fh)` with the replacement capture's first frame size, the moment `Cap::on_frame_arrived` sees it. *Why the correction:* `export::render::spans` crops the fitted picture by `letterbox((w, h), canvas)` to the pixel, and a window rect 14 px wider than its capture (the invisible DWM borders, `GetWindowRect`'s size) left a black edge either side of the picture in the owner's first two-monitor take (2026-09-14). *Why push first:* that first frame can arrive before `switch` returns.
5. `Running.video.switch(cfg, &target_id, on_size)` - the restart. On `Err` the record is removed again (`retain` on `at_ms`; there is no capture for it to describe) and the error returned.
6. Compare the rectangle with `Running.screen`, the take's own `ScreenInfo`. If they match, the take is back on the display it started on and the remap is cleared with `None` rather than replaced by an identity `Remap`, so the ordinary case costs the hook nothing. Otherwise install `Remap::for_switch((ox, oy), (w, h), (screen.origin_x, screen.origin_y), (screen.w, screen.h))` on `Running.mouse`.

*A window target switches the same way:* a window is a capture item like any other, its rect is its origin, and its frames are fitted into the canvas at whatever size WGC delivers.

*On the ordering of 5 and 6:* the remap is installed after the sink switch, not before, because until the capture has actually moved the pixels are still the old display's and raw coordinates are the correct ones. The window between the two is a few microseconds of lock-held work, against the 100 to 300 ms the new capture takes to deliver its first frame.

*What is deliberately not done:* nothing is emitted to the HUD beyond the command's result, and the take's `ScreenInfo`, encoder, canvas size and frame rate are all left alone. The one or two frames while WGC restarts are covered by the fitter's last good frame, exactly as a window resize is today.

### The switch instant

`at_ms` is taken from `recording_ms` BEFORE `VideoSink::switch` runs, not after: the replacement capture's first frame arrives while the restart is still returning, and a stamp taken afterwards sat 5 ms behind that frame in the owner's first two-monitor take (2026-09-14), so `export::render::spans` filed the new display's first frame under the old span and drew it with its bars. Stamped first, every frame the replacement capture delivers falls after the stamp - and `spans_of` opens the span on the first of them, so the restart gap itself stays on the old display's picture.
