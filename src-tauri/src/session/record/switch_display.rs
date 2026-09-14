//! Changing the recorded display (or window) while a take runs, from the HUD's Sources sheet.
//!
//! Two things move together and the take survives only if both do. The pixels move by restarting
//! the WGC capture into the SAME encoder (`gpu_restart.rs`), so `video.mp4` stays one stream at
//! the first display's size and every later frame is fitted into that canvas. The input streams
//! move by remapping their coordinates at the hook (`events::remap`), because `events.json`
//! carries one `ScreenInfo` and the export subtracts that one origin: without the remap a click
//! on the second monitor would be drawn on the first monitor's picture, hundreds of pixels away.
//!
//! Nothing is emitted to the HUD beyond the command's own result. `sync.json`'s `display_switches`
//! IS read downstream though: each entry's instant plus the new capture's `(w, h)` is what lets the
//! render crop the fitted picture out of the canvas and give the screen panel the new display's
//! aspect, instead of showing the baked black bars (`export::render::spans`).
use tauri::{AppHandle, State};

use crate::events::remap::Remap;
use crate::session::record::emit::emitter;
use crate::session::record::gpu_frames::SizeHook;
use crate::session::record::recorder::Recorder;
use crate::session::record::segments::recording_ms;
use crate::session::record::target_bounds::get_target_bounds;
use crate::session::record::video_sink::VideoStart;
use crate::session::sync::DisplaySwitch;

#[tauri::command]
pub fn switch_display(target_id: String, recorder: State<'_, Recorder>, app: AppHandle) -> Result<(), String> {
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    let running = guard.as_mut().ok_or("not recording")?;

    // Same capture settings the take started with. The encoder is already built, so `fps` only
    // sets the new capture's minimum update interval, and the cursor is never baked in (it is
    // captured as its own layer, so every style stays switchable in the editor).
    let cfg = VideoStart {
        legacy: false,
        clock: running.clock.clone(),
        stop: running.stop.clone(),
        paused: running.paused.clone(),
        totals: running.paused_totals.clone(),
        ended: emitter(&app, "record-ended-early"),
        fps: crate::win::sys::display::primary_refresh_hz().min(60),
        with_cursor: false,
    };
    // Stamped BEFORE the restart, not after it: the replacement capture's first frame arrives
    // while `switch` is still returning, and a stamp taken afterwards sat 5 ms behind that frame
    // (owner's take, 2026-09-14), so the render filed the new display's first frame under the old
    // span and drew it with its bars. Every later mouse sample is remapped from here too.
    let at_ms = recording_ms(running.clock.as_ref(), &running.paused_totals);
    // The target's rectangle: a monitor's, or a window's VISIBLE frame (`target_bounds`), which
    // is what WGC captures. The switch record starts with that size and is corrected to the
    // replacement capture's first frame the moment it arrives (`SizeHook`): the render crops the
    // fitted picture by this size to the pixel, and a window rect 14 px wider than its capture
    // left a black edge either side of the picture (owner's take, 2026-09-14). The record goes
    // in BEFORE the restart because that first frame can land before `switch` returns.
    let (w, h, ox, oy) = get_target_bounds(Some(&target_id));
    running.segments.lock().unwrap_or_else(|p| p.into_inner())
        .displays.push(DisplaySwitch { at_ms, target_id: target_id.clone(), w, h });
    let log = running.segments.clone();
    let on_size: SizeHook = Some(Box::new(move |fw, fh| {
        log.lock().unwrap_or_else(|p| p.into_inner()).set_display_size(at_ms, fw, fh);
    }));
    if let Err(e) = running.video.switch(cfg, &target_id, on_size) {
        // No replacement capture, so no frames this record could describe.
        running.segments.lock().unwrap_or_else(|p| p.into_inner()).displays.retain(|d| d.at_ms != at_ms);
        return Err(e);
    }

    // A window target switches exactly the same way: its rect is its origin, and its frames are
    // fitted into the canvas like any other size.
    let screen = running.screen;
    let same = (ox, oy) == (screen.origin_x, screen.origin_y) && (w, h) == (screen.w, screen.h);
    let remap = (!same).then(|| {
        Remap::for_switch((ox, oy), (w, h), (screen.origin_x, screen.origin_y), (screen.w, screen.h))
    });
    // Switching BACK to the take's own display clears the mapping rather than installing an
    // identity one, so the common case costs the hook nothing.
    if let Some(mouse) = running.mouse.as_ref() { mouse.set_remap(remap); }
    Ok(())
}
