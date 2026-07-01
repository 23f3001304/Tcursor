// Editor-preview support commands: the per-frame camera curve, the static layout, the click
// track, and the low-res proxy. These are the lightweight metadata the M3 editor needs to
// play the recording natively and composite a smooth, export-faithful preview on a canvas -
// kept out of preview.rs (frame compositing) so each file stays focused. All reuse the warm
// renderer cache via `with_warm`.
use std::path::PathBuf;
use crate::actions::model::ActionKind;
use crate::export::preview::{with_warm, PreviewSession};
use crate::export::render::OUT_FPS;
use crate::session::paths::ProjectPaths;
use crate::win::proc::ffcmd_bg;

/// One sample of the camera curve: output time `t` (ms), the zoom as scale + center, and the
/// cursor position - `cx`/`cy`/`curx`/`cury` are 0..1 fractions of the SCREEN content, so the
/// editor can place them on the (possibly zoomed) screen.
#[derive(serde::Serialize)]
pub struct CamSample { pub t: u32, pub scale: f32, pub cx: f32, pub cy: f32, pub curx: f32, pub cury: f32 }

/// The exact camera curve over the whole timeline, sampled per output frame. Pure math
/// (step_camera, no decode), so it is instant - the editor plays the recording natively and
/// applies this as a transform for a smooth, exact-timing zoom preview. Reuses the warm cache.
#[tauri::command]
pub fn camera_track(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<Vec<CamSample>, String> {
    with_warm(&session, &folder, |c, paths| {
        let dur = crate::edit::seed::load_or_seed(paths).trim.out_ms.max(1) as u64;
        let vs = c.meta.video_start;
        c.renderer.reset_camera();
        let step = (1000 / OUT_FPS).max(1);
        let mut out = Vec::with_capacity((dur / step + 2) as usize);
        let mut t = 0u64;
        loop {
            let pose = c.renderer.step_camera(vs + t);
            // Map the camera centre + cursor (output coords) into the screen panel rect, so
            // they are fractions of the SCREEN content - exactly what the <video> shows.
            let r = pose.scene.screen.rect;
            let rel = |vx: f32, vy: f32| (
                if r.w > 0.0 { ((vx - r.x) / r.w).clamp(0.0, 1.0) } else { 0.5 },
                if r.h > 0.0 { ((vy - r.y) / r.h).clamp(0.0, 1.0) } else { 0.5 },
            );
            let (cx, cy) = rel(pose.cam.cx, pose.cam.cy);
            let (curx, cury) = rel(pose.cur.x as f32, pose.cur.y as f32);
            out.push(CamSample { t: t as u32, scale: pose.cam.scale, cx, cy, curx, cury });
            if t >= dur { break; }
            t = (t + step).min(dur);
        }
        Ok(out)
    })
}

/// The static preview layout - screen rect + corner radius + webcam rect, as fractions of the
/// output - so the canvas preview frames the screen and webcam exactly like the export.
#[derive(serde::Serialize)]
pub struct PreviewLayout { pub screen: [f32; 4], pub radius: f32, pub cam: Option<[f32; 5]> }

#[tauri::command]
pub fn preview_layout(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<PreviewLayout, String> {
    with_warm(&session, &folder, |c, _paths| {
        c.renderer.reset_camera();
        let pose = c.renderer.step_camera(c.meta.video_start);
        let (ow, oh) = (c.out_w as f32, c.out_h as f32);
        let s = pose.scene.screen.rect;
        let cam = if pose.scene.camera.alpha > 0.5 {
            let r = pose.scene.camera.rect;
            Some([r.x / ow, r.y / oh, r.w / ow, r.h / oh, pose.scene.camera.radius / ow])
        } else { None };
        Ok(PreviewLayout { screen: [s.x / ow, s.y / oh, s.w / ow, s.h / oh], radius: pose.scene.screen.radius / ow, cam })
    })
}

/// One recorded effect-hold interval in OUTPUT time (ms) - same basis as `ClickSample`.
#[derive(serde::Serialize)]
pub struct HoldSpan { pub start_ms: u32, pub end_ms: u32 }

/// Recorded spotlight-hold intervals (the hotkey spotlight held during capture) mapped to output
/// time, so the editor preview lights the spotlight exactly like the export's `hold_alpha` - not
/// just for editor-added regions. Output basis matches `click_track`: `out = et + events_ms -
/// video_start`. Spans are clipped to `[0, dur]`; the preview applies the fade ramp.
#[tauri::command]
pub fn spotlight_holds(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<Vec<HoldSpan>, String> {
    with_warm(&session, &folder, |c, paths| {
        let dur = crate::edit::seed::load_or_seed(paths).trim.out_ms as i64;
        let off = c.renderer.events_ms() as i64 - c.meta.video_start as i64; // event_t + off = output_t
        let end_ev = (dur - off).max(0) as u32; // event time mapping to the output end
        Ok(crate::export::hold::hold_spans(c.renderer.actions(), end_ev,
            |k| matches!(k, ActionKind::SpotlightHoldStart), |k| matches!(k, ActionKind::SpotlightHoldEnd))
            .into_iter().filter_map(|(s, e)| {
                let (os, oe) = (s as i64 + off, e as i64 + off);
                (oe > 0 && os < dur).then(|| HoldSpan { start_ms: os.max(0) as u32, end_ms: oe.min(dur) as u32 })
            }).collect())
    })
}

/// One click ripple: output time `t` (ms) and 0..1 screen-content position. Same basis as
/// `CamSample`'s cursor, so the editor draws the ripple where the cursor clicked.
#[derive(serde::Serialize)]
pub struct ClickSample { pub t: u32, pub x: f32, pub y: f32 }

/// The click (mouse-down) track over the whole timeline, for click-ripple effects in the
/// editor preview that match the export's click FX. Reuses the warm renderer cache.
#[tauri::command]
pub fn click_track(folder: String, session: tauri::State<'_, PreviewSession>) -> Result<Vec<ClickSample>, String> {
    with_warm(&session, &folder, |c, _paths| {
        Ok(c.renderer.click_track(c.meta.video_start).into_iter()
            .map(|(t, x, y)| ClickSample { t, x, y }).collect())
    })
}

/// Ensure a low-res preview proxy (`preview_<h>_rt.mp4`) exists for smooth playback - the raw
/// capture is often 4K, wasteful to decode in the editor. Transcodes once (cached per height)
/// with a fast preset, no audio, faststart, and stretched (setpts) to the real recording
/// duration so the preview plays at true speed (video.mp4 is encoded sped up). Returns the path.
#[tauri::command]
pub fn ensure_proxy(folder: String, height: u32) -> Result<String, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let h = height.clamp(240, 2160) & !1; // even
    let proxy = paths.folder.join(format!("preview_{h}_rt.mp4"));
    crate::win::proc::generate_once(&proxy, || {
        // The capture encoder writes CFR at a nominal fps usually faster than the real capture
        // rate, so video.mp4 plays sped up. Stretch the proxy to the real recording duration
        // (trim.out_ms - the exact span the export spans) via setpts, so the preview plays at
        // true speed and stays aligned with the camera curve, clicks, webcam and audio.
        let real = (crate::edit::seed::load_or_seed(&paths).trim.out_ms as f64 / 1000.0).max(0.05); // assumes trim.in_ms == 0
        // A non-positive probe means the duration is unknown (ffprobe ran but couldn't parse it,
        // which returns Ok(0.0) not Err) - leave the proxy unstretched (k=1) rather than dividing
        // by the 0.05 floor and producing an absurd multi-hour stretch.
        let probed = crate::export::ffio::probe_duration(&paths.video()).unwrap_or(0.0);
        let enc = if probed > 0.05 { probed } else { real };
        let k = real / enc;
        let vf = if (k - 1.0).abs() > 0.02 { format!("scale=-2:{h},setpts={k:.6}*PTS") } else { format!("scale=-2:{h}") };
        let tmp = crate::win::proc::tmp_sibling(&proxy); // write then atomic-rename (no partial reads)
        let status = ffcmd_bg("ffmpeg")
            .args(["-v", "error", "-y", "-i"]).arg(paths.video())
            .args(["-vf", &vf, "-c:v", "libx264", "-preset", "veryfast",
                "-crf", "27", "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an"])
            .arg(&tmp)
            .status().map_err(|e| e.to_string())?;
        if !status.success() { let _ = std::fs::remove_file(&tmp); return Err("preview proxy transcode failed".into()); }
        std::fs::rename(&tmp, &proxy).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(proxy.to_string_lossy().to_string())
}
