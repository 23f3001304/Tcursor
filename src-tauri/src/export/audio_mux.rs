// Mux recorded audio tracks into the final mp4 (copy video, encode aac).
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::session::paths::ProjectPaths;

/// Combine `tmp` video with whichever of mic/system audio exist, writing
/// `final.mp4`. With both tracks they are mixed; with one it is mapped; with
/// none the temp file is moved to the destination.
pub fn mux(tmp: &Path, paths: &ProjectPaths) -> Result<()> {
    let final_path = paths.folder.join("final.mp4");
    let mic = paths.mic();
    let system = paths.system();
    let have_mic = mic.exists();
    let have_sys = system.exists();

    if have_mic && have_sys {
        run_ffmpeg(|c| {
            c.arg("-i").arg(tmp).arg("-i").arg(&mic).arg("-i").arg(&system)
                .args(["-filter_complex", "[1:a][2:a]amix=inputs=2:normalize=0[a]",
                    "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", "aac"])
                .arg(&final_path);
        })?;
    } else if have_mic || have_sys {
        let audio = if have_mic { &mic } else { &system };
        run_ffmpeg(|c| {
            c.arg("-i").arg(tmp).arg("-i").arg(audio)
                .args(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac"])
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

/// Spawn `ffmpeg -y -v error <args>` (stderr/stdout silenced) and check exit.
fn run_ffmpeg(build: impl FnOnce(&mut Command)) -> Result<()> {
    let mut cmd = Command::new("ffmpeg");
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
