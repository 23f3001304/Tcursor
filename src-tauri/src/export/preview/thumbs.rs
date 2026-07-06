// Editor-timeline media: cached ffmpeg helpers for the filmstrip thumbnails, the per-source
// audio waveform images, and a mixed preview-audio track. All mirror `ensure_proxy` (run once,
// cache by output existence). The recorder's proxy is silent; these give the editor frames to
// scrub, waveforms to show, and sound to play.
use std::path::PathBuf;
use crate::session::paths::ProjectPaths;
use crate::win::sys::proc::ffcmd_bg;

/// N evenly-spaced JPEG thumbnails (height 64) from the proxy (or raw video), cached in
/// `folder/thumbs_<count>_64/`. Returns the per-file paths (the frontend wraps each with
/// `convertFileSrc`). One ffmpeg pass: `fps=count/duration`.
#[tauri::command]
pub fn ensure_thumbs(folder: String, count: u32) -> Result<Vec<String>, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let n = count.clamp(8, 120);
    let dir = paths.folder.join(format!("thumbs_{n}_64"));
    crate::win::sys::proc::generate_once(&dir.join("thumb_0001.jpg"), || {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dur = (crate::edit::seed::load_or_seed(&paths).trim.out_ms as f64 / 1000.0).max(0.1);
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
/// Returns an empty string when that source wasn't recorded (the track just hides).
#[tauri::command]
pub fn ensure_waveform(folder: String, which: String) -> Result<String, String> {
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

/// A mixed mic+system preview-audio track (`folder/preview_audio.m4a`, AAC) so the editor can
/// play sound (the proxy is silent). Cached. NOTE: v1 mixes without the export's per-track
/// `-itsoffset`/`-ss` alignment (see `export::pipeline::audio_mux` + `exporter.rs` `shift()`), so it can
/// drift slightly vs the final render; exact-sync alignment is a follow-up.
#[tauri::command]
pub fn ensure_preview_audio(folder: String) -> Result<String, String> {
    let paths = ProjectPaths { folder: PathBuf::from(&folder) };
    let (mic, sys) = (paths.mic(), paths.system());
    let (hm, hs) = (mic.exists(), sys.exists());
    if !hm && !hs { return Ok(String::new()); }
    let out = paths.folder.join("preview_audio.m4a");
    crate::win::sys::proc::generate_once(&out, || {
        let tmp = crate::win::sys::proc::tmp_sibling(&out); // write then atomic-rename
        let mut cmd = ffcmd_bg("ffmpeg");
        cmd.args(["-v", "error", "-y"]);
        if hm { cmd.arg("-i").arg(&mic); }
        if hs { cmd.arg("-i").arg(&sys); }
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

/// Eagerly generate the editor's heavy media right after recording stops, on a background
/// thread, so opening the editor is instant instead of transcoding on open. Runs ONLY after the
/// capture threads have joined (called from the tail of `stop_recording`), so it never competes
/// with the live capture for GPU/CPU. Best-effort: each step's error is ignored (the editor's
/// lazy `ensure_*` re-attempts on open). Proxy first, so the thumbnail pass reads the small
/// proxy rather than the raw capture. The thumbnail dir is written in place (a partial dir just
/// yields fewer thumbnails for one open, then self-heals); the single-file media write
/// atomically (see `tmp_sibling`) so a mid-pre-warm editor never loads a half-written file.
pub fn prewarm(folder: String) {
    let _ = crate::export::preview::preview_track::ensure_proxy(folder.clone(), 720);
    let _ = ensure_thumbs(folder.clone(), 16);
    let _ = ensure_waveform(folder.clone(), "system".into());
    let _ = ensure_waveform(folder.clone(), "mic".into());
    let _ = ensure_preview_audio(folder);
}
