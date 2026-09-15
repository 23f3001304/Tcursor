use crate::process::proc::ffcmd_bg;
use crate::session::paths::ProjectPaths;
use std::path::PathBuf;

pub(crate) const FILMSTRIP_COUNT: u32 = 9;
pub(crate) const FILMSTRIP_HEIGHT: u32 = 80;

pub(crate) fn thumbs_spec(count: u32, height: u32) -> (u32, u32, String) {
    let n = count.clamp(8, 120);
    let h = height.clamp(16, 240) / 2 * 2;
    (n, h, format!("thumbs_{n}_{h}"))
}

#[tauri::command]
pub async fn ensure_thumbs(folder: String, count: u32, height: u32) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_thumbs_blocking(folder, count, height))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn ensure_thumbs_blocking(
    folder: String,
    count: u32,
    height: u32,
) -> Result<Vec<String>, String> {
    let paths = ProjectPaths {
        folder: PathBuf::from(&folder),
    };
    let (n, h, dir_name) = thumbs_spec(count, height);
    let dir = paths.folder.join(dir_name);
    crate::process::proc::generate_once(&dir.join("thumb_0001.jpg"), || {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dur = (crate::edit::seed::true_duration_ms(&paths) as f64 / 1000.0).max(0.1);
        let proxy = paths.folder.join("preview_720_rt.mp4");
        let from_proxy = proxy.exists();
        let src = if from_proxy { proxy } else { paths.video() };
        let mut cmd = ffcmd_bg("ffmpeg");
        cmd.args(["-v", "error", "-y"]);
        if from_proxy {
            cmd.args(["-skip_frame", "nokey"]);
        }
        let status = cmd
            .arg("-i")
            .arg(&src)
            .args([
                "-vf",
                &format!("fps={n}/{dur:.3},scale=-2:{h}"),
                "-q:v",
                "4",
            ])
            .arg(dir.join("thumb_%04d.jpg"))
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("thumbnail extraction failed".into());
        }
        Ok(())
    })?;
    let mut out = Vec::new();
    for i in 1..=n {
        let p = dir.join(format!("thumb_{i:04}.jpg"));
        if p.exists() {
            out.push(p.to_string_lossy().to_string());
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn ensure_waveform(folder: String, which: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_waveform_blocking(folder, which))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn wav_is_empty(wav: &std::path::Path) -> bool {
    std::fs::metadata(wav)
        .map(|m| m.len() <= 44)
        .unwrap_or(true)
}

pub(crate) fn ensure_waveform_blocking(folder: String, which: String) -> Result<String, String> {
    let paths = ProjectPaths {
        folder: PathBuf::from(&folder),
    };
    let wav = match which.as_str() {
        "mic" => paths.mic(),
        "system" => paths.system(),
        _ => return Err("which must be system|mic".into()),
    };
    if !wav.exists() || wav_is_empty(&wav) {
        return Ok(String::new());
    }
    let out = paths.folder.join(format!("wf_{which}.png"));
    crate::process::proc::generate_once(&out, || {
        let tmp = crate::process::proc::tmp_sibling(&out);
        let status = ffcmd_bg("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&wav)
            .args([
                "-filter_complex",
                "dynaudnorm,showwavespic=s=1180x26:colors=#6b6b86:scale=sqrt",
                "-frames:v",
                "1",
            ])
            .arg(&tmp)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            let _ = std::fs::remove_file(&tmp);
            return Err("waveform render failed".into());
        }
        std::fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(out.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn ensure_preview_audio(folder: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ensure_preview_audio_blocking(folder))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn ensure_preview_audio_blocking(folder: String) -> Result<String, String> {
    let paths = ProjectPaths {
        folder: PathBuf::from(&folder),
    };
    let (mic, sys) = (paths.mic(), paths.system());
    let (hm, hs) = (mic.exists(), sys.exists());
    if !hm && !hs {
        return Ok(String::new());
    }
    let out = paths.folder.join("preview_synced.m4a");
    let (mic_shift, sys_shift) = preview_audio_shifts(&paths);
    crate::process::proc::generate_once(&out, || {
        let tmp = crate::process::proc::tmp_sibling(&out);
        let mut cmd = ffcmd_bg("ffmpeg");
        cmd.args(["-v", "error", "-y"]);
        if hm {
            crate::export::pipeline::audio_mux::add_offset(&mut cmd, mic_shift);
            cmd.arg("-i").arg(&mic);
        }
        if hs {
            crate::export::pipeline::audio_mux::add_offset(&mut cmd, sys_shift);
            cmd.arg("-i").arg(&sys);
        }
        if hm && hs {
            cmd.args([
                "-filter_complex",
                "[0:a][1:a]amix=inputs=2:normalize=0[a]",
                "-map",
                "[a]",
            ]);
        }
        cmd.args(["-c:a", "aac"]).arg(&tmp);
        let status = cmd.status().map_err(|e| e.to_string())?;
        if !status.success() {
            let _ = std::fs::remove_file(&tmp);
            return Err("preview audio mux failed".into());
        }
        std::fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(out.to_string_lossy().to_string())
}

fn preview_audio_shifts(paths: &ProjectPaths) -> (i64, i64) {
    let log = match crate::events::model::EventLog::load(&paths.events()) {
        Ok(l) => l,
        Err(_) => return (0, 0),
    };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let vs = tl.frames.first().copied().unwrap_or(0);
    let offset = crate::edit::seed::load_or_seed(paths)
        .settings
        .audio_offset_ms as i64;
    let shift = |t| crate::export::pipeline::audio_shift_ms(t, vs, 0);
    (shift(tl.mic_ms) + offset, shift(tl.system_ms))
}

#[cfg(test)]
mod tests {
    use super::{thumbs_spec, FILMSTRIP_COUNT, FILMSTRIP_HEIGHT};

    #[test]
    fn a_thumb_spec_clamps_its_count_and_rounds_its_height_down_to_even() {
        assert_eq!(thumbs_spec(9, 80), (9, 80, "thumbs_9_80".to_string()));
        assert_eq!(thumbs_spec(2, 80).0, 8);
        assert_eq!(thumbs_spec(999, 80).0, 120);
        assert_eq!(thumbs_spec(9, 81).1, 80);
        assert_eq!(thumbs_spec(9, 4).1, 16);
        assert_eq!(thumbs_spec(9, 9999).1, 240);
    }

    #[test]
    fn the_preview_filmstrip_constants_name_the_dir_the_editor_asks_for() {
        assert_eq!(
            thumbs_spec(FILMSTRIP_COUNT, FILMSTRIP_HEIGHT).2,
            "thumbs_9_80"
        );
    }
}
