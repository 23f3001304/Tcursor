// Mux recorded audio tracks into the final output (copy video, encode audio per format).
use anyhow::{anyhow, Context, Result};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};
use crate::win::sys::proc::ffcmd;

use crate::export::pipeline::audio_segments::{segment_chain, AudioSeg};
use crate::export::settings::Format;
use crate::session::paths::ProjectPaths;

/// Combine `tmp` video with whichever of mic/system audio exist, writing `final.<ext>` (`<ext>`
/// from `format.extension()`). With both tracks they are mixed; with one it is mapped; with none
/// - or when `format` can't carry audio at all (`Format::Gif`) - the temp file is moved straight
/// to the destination. `out_dur_ms` caps the muxed audio to the (trimmed) video's own duration
/// (`-t`), so a trim-out doesn't leave a longer audio tail playing past the video's last frame;
/// unused on the no-audio path (nothing to cap).
pub fn mux(tmp: &Path, paths: &ProjectPaths, format: Format, mic_shift_ms: i64, sys_shift_ms: i64, mic_vol: f32, sys_vol: f32, out_dur_ms: u64,
           segs: &[AudioSeg]) -> Result<()> {
    let final_path = paths.folder.join(format!("final.{}", format.extension()));
    let mic = paths.mic();
    let system = paths.system();
    let have_mic = format.supports_audio() && mic.exists();
    let have_sys = format.supports_audio() && system.exists();

    if have_mic && have_sys {
        let acodec = format.audio_codec();
        let tracks = [
            AudioTrack { path: &mic, shift_ms: mic_shift_ms, vol: mic_vol },
            AudioTrack { path: &system, shift_ms: sys_shift_ms, vol: sys_vol },
        ];
        run_ffmpeg(&mux_args(tmp, &tracks, acodec, out_dur_ms, &final_path, segs))?;
    } else if have_mic || have_sys {
        let acodec = format.audio_codec();
        let (audio, shift, vol) = if have_mic { (&mic, mic_shift_ms, mic_vol) } else { (&system, sys_shift_ms, sys_vol) };
        let tracks = [AudioTrack { path: audio, shift_ms: shift, vol }];
        run_ffmpeg(&mux_args(tmp, &tracks, acodec, out_dur_ms, &final_path, segs))?;
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

/// One audio input to `mux_args`: its file path, signed A/V shift (`add_offset`'s sign
/// convention), and linear volume gain.
struct AudioTrack<'a> {
    path: &'a Path,
    shift_ms: i64,
    vol: f32,
}

/// Build the `ffmpeg` args (appended after the shared `-y -v error`) that mux `tmp`'s video
/// with 1 or 2 audio tracks into `final_path`, pure (no process spawn) so the mix-filter vs.
/// single-map branch choice and the `-t` duration cap are unit-testable without launching
/// ffmpeg. `tracks.len()` must be 1 or 2 - `mux`'s own branching guarantees this (the 0-track
/// case takes the separate rename path and never reaches here). `-t <out_dur_ms>` is appended
/// last, immediately before `final_path`, so the muxed output is capped to the (trimmed)
/// video's own duration regardless of how long the source audio file actually is.
/// `segs` is the time remap's kept ranges (`audio_segments::audio_segs`); the identity (one
/// segment at 1x, or none) adds nothing, so a no-cut export runs the exact pre-remap command.
fn mux_args(tmp: &Path, tracks: &[AudioTrack], acodec: &str, out_dur_ms: u64, final_path: &Path, segs: &[AudioSeg]) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["-i".into(), tmp.as_os_str().to_os_string()];
    for t in tracks {
        args.extend(offset_args(t.shift_ms));
        args.push("-i".into());
        args.push(t.path.as_os_str().to_os_string());
    }
    // With a segment chain the mix (or the lone track) lands on `[x]` and the chain carries it to
    // `[a]`; without one the labels and flags are exactly what they were before the time remap.
    let chain = segment_chain("[x]", segs, "[a]");
    if tracks.len() == 2 {
        let mut filter = format!(
            "[1:a]volume={}[m];[2:a]volume={}[s];[m][s]amix=inputs=2:normalize=0{}",
            tracks[0].vol, tracks[1].vol, if chain.is_some() { "[x]" } else { "[a]" }
        );
        if let Some(c) = &chain { filter.push(';'); filter.push_str(c); }
        args.extend(["-filter_complex", &filter, "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", acodec].map(OsString::from));
    } else if let Some(c) = &chain {
        let filter = format!("[1:a]volume={}[x];{c}", tracks[0].vol);
        args.extend(["-filter_complex", &filter, "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", acodec].map(OsString::from));
    } else {
        let af = format!("volume={}", tracks[0].vol);
        args.extend(["-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", acodec, "-af", &af].map(OsString::from));
    }
    args.push("-t".into());
    args.push(format!("{:.3}", out_dur_ms as f64 / 1000.0).into());
    args.push(final_path.as_os_str().to_os_string());
    args
}

/// Align an audio input to the video start: positive shift = delay
/// (`-itsoffset`), negative = trim the lead (`-ss`). Emitted before its `-i`.
/// `pub(crate)` so the editor's preview-audio mux (`preview::thumbs`) applies the exact same
/// alignment the final render does, keeping preview playback in sync.
pub(crate) fn add_offset(c: &mut Command, shift_ms: i64) {
    c.args(offset_args(shift_ms));
}

/// Pure arg list for `add_offset`'s alignment (same sign convention) - shared with `mux_args`
/// so both the `Command`-builder and the `Vec<OsString>`-builder emit identical args.
fn offset_args(shift_ms: i64) -> Vec<OsString> {
    if shift_ms > 0 {
        vec!["-itsoffset".into(), format!("{:.3}", shift_ms as f64 / 1000.0).into()]
    } else if shift_ms < 0 {
        vec!["-ss".into(), format!("{:.3}", (-shift_ms) as f64 / 1000.0).into()]
    } else {
        vec![]
    }
}

/// Spawn `ffmpeg -y -v error <args>` (stderr/stdout silenced) and check exit.
fn run_ffmpeg(args: &[OsString]) -> Result<()> {
    let status = ffcmd("ffmpeg")
        .args(["-y", "-v", "error"])
        .args(args)
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
#[path = "audio_mux_tests.rs"]
mod tests;
