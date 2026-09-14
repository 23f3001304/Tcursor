// Editor-preview support commands: the per-frame camera curve, the static layout, the click
// track, and the low-res proxy. These are the lightweight metadata the M3 editor needs to
// play the recording natively and composite a smooth, export-faithful preview on a canvas -
// kept out of preview.rs (frame compositing) so each file stays focused. All reuse the warm
// renderer cache via `with_warm`.
use std::io::BufRead;
use std::path::PathBuf;
use crate::export::preview::{with_warm, PreviewSession};
use crate::export::render::{OUT_FPS, OUT_STEP_MS};
use crate::session::paths::ProjectPaths;
use crate::win::sys::proc::ffcmd_bg;

/// One sample of the camera curve: output time `t` (ms), the zoom as scale + center, and the
/// cursor position - `cx`/`cy`/`curx`/`cury` are 0..1 fractions of the SCREEN content, so the
/// editor can place them on the (possibly zoomed) screen.
#[derive(serde::Serialize)]
pub struct CamSample { pub t: u32, pub scale: f32, pub cx: f32, pub cy: f32, pub curx: f32, pub cury: f32 }

/// The exact camera curve over the whole timeline, sampled per output frame. On a warm cache the
/// body is pure math (step_camera, no decode) - but it is O(clip length) at `OUT_FPS`, and a cold
/// cache runs the full `FrameRenderer::new`, so like every other `with_warm` command it is
/// `async` + `spawn_blocking` (see `preview_frame` for the pattern and the `AppHandle` reason).
#[tauri::command]
pub async fn camera_track(folder: String, app: tauri::AppHandle) -> Result<Vec<CamSample>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        with_warm(&app.state::<PreviewSession>(), &folder, |c, _paths| {
            // The TRUE full clip length (not `trim.out_ms`, which once a user actually trims is a
            // strict sub-range) - so scrubbing into a trimmed-out region still shows an animated
            // curve instead of freezing on the last in-range sample.
            let vs = c.meta.video_start;
            c.renderer.reset_camera();
            let plan = c.renderer.time_map().frame_plan(OUT_FPS);
            let mut out = Vec::with_capacity(plan.len());
            c.renderer.walk_plan(vs, OUT_FPS, &plan, plan.len(), OUT_STEP_MS, |_, j, _, pose| {
                let r = pose.scene.screen.rect;
                let rel = |vx: f32, vy: f32| (
                    if r.w > 0.0 { ((vx - r.x) / r.w).clamp(0.0, 1.0) } else { 0.5 },
                    if r.h > 0.0 { ((vy - r.y) / r.h).clamp(0.0, 1.0) } else { 0.5 },
                );
                let (cx, cy) = rel(pose.cam.cx, pose.cam.cy);
                let (curx, cury) = rel(pose.cur.x as f32, pose.cur.y as f32);
                out.push(CamSample { t: (j as u64 * 1000 / OUT_FPS) as u32, scale: pose.cam.scale, cx, cy, curx, cury });
                true
            });
            Ok(out)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The static preview layout - screen rect + corner radius + webcam rect, as fractions of the
/// output - so the canvas preview frames the screen and webcam exactly like the export. `cam`'s
/// last 4 entries are the camera panel's ring: width (fraction of output width, 0 = no ring)
/// then RGB 0..255, mirroring how `Panel::ring_px`/`ring_color` ride alongside its rect/radius.
/// `canvas` is the resolved preview frame's pixel dimensions (`Layout::resolve`'s output) - the
/// editor sizes its canvas + `.e-stage` aspect-ratio from this instead of a hardcoded 16:9.
#[derive(serde::Serialize)]
pub struct PreviewLayout { pub screen: [f32; 4], pub radius: f32, pub cam: Option<[f32; 9]>, pub canvas: [u32; 2] }

/// `async` + `spawn_blocking` for the same reason as `camera_track`: a cold cache runs the full
/// `FrameRenderer::new` inside `with_warm`, which must never land on the main thread.
#[tauri::command]
pub async fn preview_layout(folder: String, app: tauri::AppHandle) -> Result<PreviewLayout, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        with_warm(&app.state::<PreviewSession>(), &folder, |c, _paths| {
            c.renderer.reset_camera();
            let pose = c.renderer.step_camera(c.meta.video_start, 0, OUT_STEP_MS);
            let (ow, oh) = (c.meta.out_w as f32, c.meta.out_h as f32);
            let s = pose.scene.screen.rect;
            let cam = if pose.scene.camera.alpha > 0.5 {
                let cp = pose.scene.camera;
                let [rr, rg, rb] = cp.ring_color;
                Some([cp.rect.x / ow, cp.rect.y / oh, cp.rect.w / ow, cp.rect.h / oh, cp.radius / ow,
                    cp.ring_px / ow, rr as f32, rg as f32, rb as f32])
            } else { None };
            Ok(PreviewLayout { screen: [s.x / ow, s.y / oh, s.w / ow, s.h / oh], radius: pose.scene.screen.radius / ow, cam,
                canvas: [c.meta.out_w, c.meta.out_h] })
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// One click ripple: output time `t` (ms) and 0..1 screen-content position. Same basis as
/// `CamSample`'s cursor, so the editor draws the ripple where the cursor clicked.
#[derive(serde::Serialize)]
pub struct ClickSample { pub t: u32, pub x: f32, pub y: f32 }

/// The click (mouse-down) track over the whole timeline, for click-ripple effects in the
/// editor preview that match the export's click FX. Reuses the warm renderer cache, so it is
/// `async` + `spawn_blocking` like every other `with_warm` command (cold build = subprocesses).
#[tauri::command]
pub async fn click_track(folder: String, app: tauri::AppHandle) -> Result<Vec<ClickSample>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        with_warm(&app.state::<PreviewSession>(), &folder, |c, _paths| {
            Ok(c.renderer.click_track(c.meta.video_start).into_iter()
                .map(|(t, x, y)| ClickSample { t, x, y }).collect())
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Ensure a low-res preview proxy (`preview_<h>_rt.mp4`) exists for smooth playback - the raw
/// capture is often 4K, wasteful to decode in the editor. Transcodes once (cached per height)
/// with a fast preset, no audio, faststart, and stretched (setpts) to the real recording
/// duration so the preview plays at true speed (video.mp4 is encoded sped up). Returns the path.
/// `async` + `spawn_blocking` - same freeze mechanism as `ai::commands` (Task 40) and
/// `thumbs::ensure_thumbs` (Task 41): a sync `#[tauri::command] fn` would run the blocking ffmpeg
/// `.status()` call inline on the main thread, freezing the window for the transcode's duration
/// (a project OPEN, since this is the essential preprocessing step). The blocking body is
/// `ensure_proxy_blocking`, called directly (no runtime hop needed) by `preprocess::run`, which
/// already runs off the main thread on its own `std::thread`.
#[tauri::command]
pub async fn ensure_proxy(folder: String, height: u32) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_proxy_blocking(folder, height))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn ensure_proxy_blocking(folder: String, height: u32) -> Result<String, String> {
    ensure_proxy_with_progress(folder, height, &|_| {})
}

/// `ensure_proxy_blocking` that reports the transcode's own progress - 0..99 from ffmpeg's
/// `-progress` stream against the real recording duration (`proxy_pct`) - because the proxy IS
/// the wait between Stop and the editor (`preprocess::essential`), and a pill that sat at 0% for
/// a minute on a long take read as hung.
pub(crate) fn ensure_proxy_with_progress(folder: String, height: u32, on_progress: &dyn Fn(u32)) -> Result<String, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let h = height.clamp(240, 2160) & !1; // even
    let proxy = paths.folder.join(format!("preview_{h}_rt.mp4"));
    crate::win::sys::proc::generate_once(&proxy, || {
        // The capture encoder writes CFR at a nominal fps usually faster than the real capture
        // rate, so video.mp4 plays sped up. Stretch the proxy to the real recording duration via
        // setpts, so the preview plays at true speed and stays aligned with the camera curve,
        // clicks, webcam and audio. Uses the TRUE full duration (not `trim.out_ms`, which once a
        // user actually trims no longer spans the whole clip) - the proxy covers the entire
        // scrubbable timeline, trimmed or not.
        let real = (crate::edit::seed::true_duration_ms(&paths) as f64 / 1000.0).max(0.05);
        // A non-positive probe means the duration is unknown (ffprobe ran but couldn't parse it,
        // which returns Ok(0.0) not Err) - leave the proxy unstretched (k=1) rather than dividing
        // by the 0.05 floor and producing an absurd multi-hour stretch.
        let probed = crate::export::pipeline::ffio::probe_duration(&paths.video()).unwrap_or(0.0);
        let enc = if probed > 0.05 { probed } else { real };
        let k = real / enc;
        let vf = if (k - 1.0).abs() > 0.02 { format!("scale=-2:{h},setpts={k:.6}*PTS") } else { format!("scale=-2:{h}") };
        let tmp = crate::win::sys::proc::tmp_sibling(&proxy); // write then atomic-rename (no partial reads)
        // No `-hwaccel`: HW decode into the CPU `scale`/`setpts` filters measured 3x SLOWER than
        // CPU decode on 1080p60 (the surface download), and `auto` once picked a frame format the
        // filters rejected outright. CPU decode is the floor here; `ultrafast` (a third quicker
        // than `veryfast` at this size, and a preview does not need the compression) keeps the
        // encode under it. `-g 60` puts a keyframe every second so the editor's seeks land fast
        // and the filmstrip can read keyframes only (`thumbs::ensure_thumbs_blocking`).
        let mut child = ffcmd_bg("ffmpeg")
            .args(["-v", "error", "-nostats", "-progress", "pipe:1", "-y", "-i"]).arg(paths.video())
            .args(["-vf", &vf, "-c:v", "libx264", "-preset", "ultrafast", "-g", "60",
                "-crf", "27", "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an"])
            .arg(&tmp)
            .stdout(std::process::Stdio::piped())
            .spawn().map_err(|e| e.to_string())?;
        if let Some(out) = child.stdout.take() {
            for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
                if let Some(pct) = proxy_pct(&line, real) { on_progress(pct); }
            }
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() { let _ = std::fs::remove_file(&tmp); return Err("preview proxy transcode failed".into()); }
        std::fs::rename(&tmp, &proxy).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(proxy.to_string_lossy().to_string())
}

/// One line of ffmpeg's `-progress` stream -> percent of the proxy written, for `out_time_us=`
/// lines only (`None` for the rest of its key=value chatter). Output time is already stretched to
/// the real duration by `setpts`, so it is measured against `real_secs`. Capped at 99: the last
/// point is the caller's, once the file has been renamed into place.
pub(crate) fn proxy_pct(line: &str, real_secs: f64) -> Option<u32> {
    let us: f64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    Some(((us / 1e6 / real_secs.max(0.05)) * 100.0).clamp(0.0, 99.0) as u32)
}
