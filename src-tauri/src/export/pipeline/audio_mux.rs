// Mux recorded audio tracks into the final mp4 (copy video, encode aac).
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use crate::win::sys::proc::ffcmd;

use crate::session::paths::ProjectPaths;

/// Combine `tmp` video with whichever of mic/system audio exist, writing
/// `final.mp4`. With both tracks they are mixed; with one it is mapped; with
/// none the temp file is moved to the destination.
pub fn mux(tmp: &Path, paths: &ProjectPaths, mic_shift_ms: i64, sys_shift_ms: i64) -> Result<()> {
    let final_path = paths.folder.join("final.mp4");
    let mic = paths.mic();
    let system = paths.system();
    let have_mic = mic.exists();
    let have_sys = system.exists();

    if have_mic && have_sys {
        run_ffmpeg(|c| {
            c.arg("-i").arg(tmp);
            add_offset(c, mic_shift_ms); c.arg("-i").arg(&mic);
            add_offset(c, sys_shift_ms); c.arg("-i").arg(&system);
            c.args(["-filter_complex", "[1:a][2:a]amix=inputs=2:normalize=0[a]",
                "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", "aac"])
                .arg(&final_path);
        })?;
    } else if have_mic || have_sys {
        let (audio, shift) = if have_mic { (&mic, mic_shift_ms) } else { (&system, sys_shift_ms) };
        run_ffmpeg(|c| {
            c.arg("-i").arg(tmp);
            add_offset(c, shift); c.arg("-i").arg(audio);
            c.args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac"])
                .arg(&final_path);
        })?;
    } else {
        // No audio: move the muxed video into place.
        if final_path.exists() { std::fs::remove_file(&final_path).ok(); }
        std::fs::rename(tmp, &final_path).context("rename tmp -> final.mp4")?;
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
