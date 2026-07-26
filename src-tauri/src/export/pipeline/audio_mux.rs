// Mux recorded audio tracks into the final output (copy video, encode audio per format).
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use crate::win::sys::proc::ffcmd;

use crate::export::settings::Format;
use crate::session::paths::ProjectPaths;

/// Combine `tmp` video with whichever of mic/system audio exist, writing `final.<ext>` (`<ext>`
/// from `format.extension()`). With both tracks they are mixed; with one it is mapped; with none
/// - or when `format` can't carry audio at all (`Format::Gif`) - the temp file is moved straight
/// to the destination.
pub fn mux(tmp: &Path, paths: &ProjectPaths, format: Format, mic_shift_ms: i64, sys_shift_ms: i64, mic_vol: f32, sys_vol: f32) -> Result<()> {
    let final_path = paths.folder.join(format!("final.{}", format.extension()));
    let mic = paths.mic();
    let system = paths.system();
    let have_mic = format.supports_audio() && mic.exists();
    let have_sys = format.supports_audio() && system.exists();

    if have_mic && have_sys {
        let acodec = format.audio_codec();
        run_ffmpeg(|c| {
            c.arg("-i").arg(tmp);
            add_offset(c, mic_shift_ms); c.arg("-i").arg(&mic);
            add_offset(c, sys_shift_ms); c.arg("-i").arg(&system);
            let filter = format!("[1:a]volume={mic_vol}[m];[2:a]volume={sys_vol}[s];[m][s]amix=inputs=2:normalize=0[a]");
            c.args(["-filter_complex", &filter, "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", acodec])
                .arg(&final_path);
        })?;
    } else if have_mic || have_sys {
        let acodec = format.audio_codec();
        let (audio, shift, vol) = if have_mic { (&mic, mic_shift_ms, mic_vol) } else { (&system, sys_shift_ms, sys_vol) };
        run_ffmpeg(|c| {
            c.arg("-i").arg(tmp);
            add_offset(c, shift); c.arg("-i").arg(audio);
            c.args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", acodec, "-af", &format!("volume={vol}")])
                .arg(&final_path);
        })?;
    } else {
        // No audio (nothing recorded, or `format` can't carry it - e.g. `Gif`): move the
        // encoded video/image into place.
        if final_path.exists() { std::fs::remove_file(&final_path).ok(); }
        std::fs::rename(tmp, &final_path).context("rename tmp -> final")?;
        return Ok(());
    }

    std::fs::remove_file(tmp).ok();
    Ok(())
}

/// Align an audio input to the video start: positive shift = delay
/// (`-itsoffset`), negative = trim the lead (`-ss`). Emitted before its `-i`.
fn add_offset(c: &mut Command, shift_ms: i64) {
    if shift_ms > 0 {
        c.args(["-itsoffset", &format!("{:.3}", shift_ms as f64 / 1000.0)]);
    } else if shift_ms < 0 {
        c.args(["-ss", &format!("{:.3}", (-shift_ms) as f64 / 1000.0)]);
    }
}

/// Spawn `ffmpeg -y -v error <args>` (stderr/stdout silenced) and check exit.
fn run_ffmpeg(build: impl FnOnce(&mut Command)) -> Result<()> {
    let mut cmd = ffcmd("ffmpeg");
    cmd.args(["-y", "-v", "error"]);
    build(&mut cmd);
    let status = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("spawn ffmpeg (mux)")?;
    if !status.success() {
        return Err(anyhow!("ffmpeg mux exited with {status}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh temp folder for one test (never touches a real recording).
    fn tmp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tcursor-mux-test-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn no_audio_renames_tmp_to_final_with_the_format_extension() {
        let folder = tmp_dir("mp4");
        let paths = ProjectPaths { folder: folder.clone() };
        let tmp = folder.join("tmp_export.mp4");
        std::fs::write(&tmp, b"fake video bytes").unwrap();
        mux(&tmp, &paths, Format::Mp4, 0, 0, 1.0, 1.0).expect("mux");
        assert!(folder.join("final.mp4").exists());
        assert!(!tmp.exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// `Gif` can never carry audio, so `mux` must take the rename-only path even when mic audio
    /// was actually recorded for this session - back-compat for `have_mic`/`have_sys` now being
    /// gated on `format.supports_audio()`.
    #[test]
    fn gif_always_takes_the_no_audio_path_even_with_recorded_audio() {
        let folder = tmp_dir("gif");
        let paths = ProjectPaths { folder: folder.clone() };
        let tmp = folder.join("tmp_export.gif");
        std::fs::write(&tmp, b"fake gif bytes").unwrap();
        std::fs::write(paths.mic(), b"fake mic wav").unwrap();
        mux(&tmp, &paths, Format::Gif, 0, 0, 1.0, 1.0).expect("mux");
        assert!(folder.join("final.gif").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn webm_final_path_uses_the_webm_extension() {
        let folder = tmp_dir("webm");
        let paths = ProjectPaths { folder: folder.clone() };
        let tmp = folder.join("tmp_export.webm");
        std::fs::write(&tmp, b"fake webm bytes").unwrap();
        mux(&tmp, &paths, Format::WebM, 0, 0, 1.0, 1.0).expect("mux");
        assert!(folder.join("final.webm").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
