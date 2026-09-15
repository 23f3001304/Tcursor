use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::events::remap::Remap;
use crate::platform::Platform;
use crate::ports::capture::{CaptureRequest, FirstFrameSize, TargetId};
use crate::session::paths::ProjectPaths;
use crate::session::record::emit::emitter;
use crate::session::record::recorder::Recorder;
use crate::session::record::segments::recording_ms;
use crate::session::sync::DisplaySwitch;

#[tauri::command]
pub fn switch_display(
    target_id: String,
    recorder: State<'_, Recorder>,
    platform: State<'_, Arc<Platform>>,
    app: AppHandle,
) -> Result<(), String> {
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    let running = guard.as_mut().ok_or("not recording")?;

    let target = TargetId::from_arg(Some(&target_id));
    let paths = ProjectPaths {
        folder: std::path::PathBuf::from(&running.folder),
    };
    let req = CaptureRequest {
        target,
        output: paths.video(),
        fps: platform.system.primary_refresh_hz().min(60),
        with_cursor: false,
        prefer_compatibility: false,
        clock: running.clock.clone(),
        stop: running.stop.clone(),
        paused: running.paused.clone(),
        totals: running.paused_totals.clone(),
        ended: emitter(&app, "record-ended-early"),
    };
    let at_ms = recording_ms(running.clock.as_ref(), &running.paused_totals);
    let g = platform.capture.bounds(&target);
    running
        .segments
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .displays
        .push(DisplaySwitch {
            at_ms,
            target_id: target_id.clone(),
            w: g.w,
            h: g.h,
        });
    let log = running.segments.clone();
    let on_size: FirstFrameSize = Some(Box::new(move |fw, fh| {
        log.lock()
            .unwrap_or_else(|p| p.into_inner())
            .set_display_size(at_ms, fw, fh);
    }));
    if let Err(e) = running.video.switch(req, on_size) {
        running
            .segments
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .displays
            .retain(|d| d.at_ms != at_ms);
        return Err(e);
    }

    let screen = running.screen;
    let same = (g.origin_x, g.origin_y) == (screen.origin_x, screen.origin_y)
        && (g.w, g.h) == (screen.w, screen.h);
    let remap = (!same).then(|| {
        Remap::for_switch(
            (g.origin_x, g.origin_y),
            (g.w, g.h),
            (screen.origin_x, screen.origin_y),
            (screen.w, screen.h),
        )
    });
    running.mouse.set_remap(remap);
    Ok(())
}
