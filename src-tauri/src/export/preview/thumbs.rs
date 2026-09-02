// Editor-timeline media: cached ffmpeg helpers for the filmstrip thumbnails, the per-source
// audio waveform images, and a mixed preview-audio track. All mirror `ensure_proxy` (run once,
// cache by output existence). The recorder's proxy is silent; these give the editor frames to
// scrub, waveforms to show, and sound to play.
use std::path::PathBuf;
use crate::session::paths::ProjectPaths;
use crate::win::sys::proc::ffcmd_bg;

/// N evenly-spaced JPEG thumbnails (height 64) from the proxy (or raw video), cached in
/// `folder/thumbs_<count>_64/`. Returns the per-file paths (the frontend wraps each with
/// `convertFileSrc`). One ffmpeg pass: `fps=count/duration`. `async` + `spawn_blocking` - same
/// freeze mechanism as `ai::commands` (Task 40): a sync `#[tauri::command] fn` runs the blocking
/// ffmpeg `.status()` call inline on the main thread, freezing the window for the pass's
/// duration. The blocking body is `ensure_thumbs_blocking`, called directly (no runtime hop
/// needed) by `preprocess::run`, which already runs off the main thread on its own `std::thread`.
#[tauri::command]
pub async fn ensure_thumbs(folder: String, count: u32) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_thumbs_blocking(folder, count))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn ensure_thumbs_blocking(folder: String, count: u32) -> Result<Vec<String>, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let n = count.clamp(8, 120);
    let dir = paths.folder.join(format!("thumbs_{n}_64"));
    crate::win::sys::proc::generate_once(&dir.join("thumb_0001.jpg"), || {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        // TRUE full duration, not `trim.out_ms` (a sub-range once a user actually trims) - the
        // filmstrip spans the whole scrubbable timeline, trimmed or not.
        let dur = (crate::edit::seed::true_duration_ms(&paths) as f64 / 1000.0).max(0.1);
        let proxy = paths.folder.join("preview_720_rt.mp4");
        let src = if proxy.exists() { proxy } else { paths.video() };
        let status = ffcmd_bg("ffmpeg")
            .args(["-v", "error", "-y", "-i"]).arg(&src)
            .args(["-vf", &format!("fps={n}/{dur:.3},scale=-2:64"), "-q:v", "4"])
            .arg(dir.join("thumb_%04d.jpg"))
            .status().map_err(|e| e.to_string())?;
        if !status.success() { return Err("thumbnail extraction failed".into()); }
        Ok(())
    })?;
    let mut out = Vec::new();
    for i in 1..=n {
        let p = dir.join(format!("thumb_{i:04}.jpg"));
        if p.exists() { out.push(p.to_string_lossy().to_string()); }
    }
    Ok(out)
}

/// A waveform PNG for one source (`"system"` or `"mic"`), cached as `folder/wave_<which>.png`.
/// Returns an empty string when that source wasn't recorded (the track just hides). `async` +
/// `spawn_blocking` - see `ensure_thumbs` above; `ensure_waveform_blocking` is the direct callee
/// for `preprocess::run`.
#[tauri::command]
pub async fn ensure_waveform(folder: String, which: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_waveform_blocking(folder, which))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn ensure_waveform_blocking(folder: String, which: String) -> Result<String, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let wav = match which.as_str() {
        "mic" => paths.mic(),
        "system" => paths.system(),
        _ => return Err("which must be system|mic".into()),
    };
    if !wav.exists() { return Ok(String::new()); }
    let out = paths.folder.join(format!("wf_{which}.png"));
    crate::win::sys::proc::generate_once(&out, || {
        let tmp = crate::win::sys::proc::tmp_sibling(&out); // write then atomic-rename
        // dynaudnorm normalizes loudness and scale=sqrt emphasizes low amplitudes, so a quiet mic
        // shows a visible waveform (a flat, invisible line otherwise) even though the export mic is
        // audible. The `wf_` prefix invalidates older flat `wave_*.png` caches.
        let status = ffcmd_bg("ffmpeg")
            .args(["-v", "error", "-y", "-i"]).arg(&wav)
            .args(["-filter_complex", "dynaudnorm,showwavespic=s=1180x26:colors=#6b6b86:scale=sqrt", "-frames:v", "1"])
            .arg(&tmp)
            .status().map_err(|e| e.to_string())?;
        if !status.success() { let _ = std::fs::remove_file(&tmp); return Err("waveform render failed".into()); }
        std::fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(out.to_string_lossy().to_string())
}

/// A mixed mic+system preview-audio track (`folder/preview_synced.m4a`, AAC) so the editor can play
/// sound (the proxy is silent). Cached. Each track is shifted to the video start with the SAME
/// per-track `-itsoffset`/`-ss` alignment the final render uses (`audio_mux::add_offset`), so preview
/// playback stays in sync with the (re-timed) proxy instead of drifting by the capture-warmup lead.
/// The `preview_synced.` name (vs the old `preview_audio.`) invalidates stale un-aligned caches.
/// `async` + `spawn_blocking` - see `ensure_thumbs` above; `ensure_preview_audio_blocking` is the
/// direct callee for `preprocess::run`.
#[tauri::command]
pub async fn ensure_preview_audio(folder: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_preview_audio_blocking(folder))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn ensure_preview_audio_blocking(folder: String) -> Result<String, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let (mic, sys) = (paths.mic(), paths.system());
    let (hm, hs) = (mic.exists(), sys.exists());
    if !hm && !hs { return Ok(String::new()); }
    let out = paths.folder.join("preview_synced.m4a");
    let (mic_shift, sys_shift) = preview_audio_shifts(&paths);
    crate::win::sys::proc::generate_once(&out, || {
        let tmp = crate::win::sys::proc::tmp_sibling(&out); // write then atomic-rename
        let mut cmd = ffcmd_bg("ffmpeg");
        cmd.args(["-v", "error", "-y"]);
        if hm { crate::export::pipeline::audio_mux::add_offset(&mut cmd, mic_shift); cmd.arg("-i").arg(&mic); }
        if hs { crate::export::pipeline::audio_mux::add_offset(&mut cmd, sys_shift); cmd.arg("-i").arg(&sys); }
        if hm && hs {
            cmd.args(["-filter_complex", "[0:a][1:a]amix=inputs=2:normalize=0[a]", "-map", "[a]"]);
        }
        cmd.args(["-c:a", "aac"]).arg(&tmp);
        let status = cmd.status().map_err(|e| e.to_string())?;
        if !status.success() { let _ = std::fs::remove_file(&tmp); return Err("preview audio mux failed".into()); }
        std::fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(out.to_string_lossy().to_string())
}

/// Per-track mic/system shift (ms) to align preview audio to the video's frame 0: `track_start -
/// video_start` (+ the mic audio-offset), matching `exporter::export`'s `shift()` minus the trim (the
/// preview plays the whole clip; trim is a playback clamp). `(0, 0)` if the timeline can't be loaded.
fn preview_audio_shifts(paths: &ProjectPaths) -> (i64, i64) {
    let log = match crate::events::model::EventLog::load(&paths.events()) { Ok(l) => l, Err(_) => return (0, 0) };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let vs = tl.frames.first().copied().unwrap_or(0);
    let offset = crate::edit::seed::load_or_seed(paths).settings.audio_offset_ms as i64;
    // The same seam the export mux uses (`pipeline::audio_shift_ms`) with a zero trim, rather
    // than a second copy of the formula: the preview plays the WHOLE clip and applies trim as a
    // playback clamp, so only the export has a (frame-floored) trim-in to subtract here.
    let shift = |t| crate::export::pipeline::audio_shift_ms(t, vs, 0);
    (shift(tl.mic_ms) + offset, shift(tl.system_ms))
}
