use crate::process::proc::ffcmd;
use anyhow::{anyhow, Context, Result};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::export::pipeline::audio_segments::{segment_chain, AudioSeg};
use crate::export::settings::Format;
use crate::session::paths::ProjectPaths;

pub fn mux(
    tmp: &Path,
    paths: &ProjectPaths,
    format: Format,
    mic_shift_ms: i64,
    sys_shift_ms: i64,
    mic_vol: f32,
    sys_vol: f32,
    out_dur_ms: u64,
    segs: &[AudioSeg],
) -> Result<()> {
    let final_path = paths.folder.join(format!("final.{}", format.extension()));
    let mic = paths.mic();
    let system = paths.system();
    let have_mic = format.supports_audio() && mic.exists();
    let have_sys = format.supports_audio() && system.exists();

    if have_mic && have_sys {
        let acodec = format.audio_codec();
        let tracks = [
            AudioTrack {
                path: &mic,
                shift_ms: mic_shift_ms,
                vol: mic_vol,
            },
            AudioTrack {
                path: &system,
                shift_ms: sys_shift_ms,
                vol: sys_vol,
            },
        ];
        run_ffmpeg(&mux_args(
            tmp,
            &tracks,
            acodec,
            out_dur_ms,
            &final_path,
            segs,
        ))?;
    } else if have_mic || have_sys {
        let acodec = format.audio_codec();
        let (audio, shift, vol) = if have_mic {
            (&mic, mic_shift_ms, mic_vol)
        } else {
            (&system, sys_shift_ms, sys_vol)
        };
        let tracks = [AudioTrack {
            path: audio,
            shift_ms: shift,
            vol,
        }];
        run_ffmpeg(&mux_args(
            tmp,
            &tracks,
            acodec,
            out_dur_ms,
            &final_path,
            segs,
        ))?;
    } else {
        if final_path.exists() {
            std::fs::remove_file(&final_path).ok();
        }
        std::fs::rename(tmp, &final_path).context("rename tmp -> final")?;
        return Ok(());
    }

    std::fs::remove_file(tmp).ok();
    Ok(())
}

struct AudioTrack<'a> {
    path: &'a Path,
    shift_ms: i64,
    vol: f32,
}

fn mux_args(
    tmp: &Path,
    tracks: &[AudioTrack],
    acodec: &str,
    out_dur_ms: u64,
    final_path: &Path,
    segs: &[AudioSeg],
) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["-i".into(), tmp.as_os_str().to_os_string()];
    for t in tracks {
        args.extend(offset_args(t.shift_ms));
        args.push("-i".into());
        args.push(t.path.as_os_str().to_os_string());
    }
    let chain = segment_chain("[x]", segs, "[a]");
    if tracks.len() == 2 {
        let mut filter = format!(
            "[1:a]volume={}[m];[2:a]volume={}[s];[m][s]amix=inputs=2:normalize=0{}",
            tracks[0].vol,
            tracks[1].vol,
            if chain.is_some() { "[x]" } else { "[a]" }
        );
        if let Some(c) = &chain {
            filter.push(';');
            filter.push_str(c);
        }
        args.extend(
            [
                "-filter_complex",
                &filter,
                "-map",
                "0:v",
                "-map",
                "[a]",
                "-c:v",
                "copy",
                "-c:a",
                acodec,
            ]
            .map(OsString::from),
        );
    } else if let Some(c) = &chain {
        let filter = format!("[1:a]volume={}[x];{c}", tracks[0].vol);
        args.extend(
            [
                "-filter_complex",
                &filter,
                "-map",
                "0:v",
                "-map",
                "[a]",
                "-c:v",
                "copy",
                "-c:a",
                acodec,
            ]
            .map(OsString::from),
        );
    } else {
        let af = format!("volume={}", tracks[0].vol);
        args.extend(
            [
                "-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", acodec, "-af", &af,
            ]
            .map(OsString::from),
        );
    }
    args.push("-t".into());
    args.push(format!("{:.3}", out_dur_ms as f64 / 1000.0).into());
    args.push(final_path.as_os_str().to_os_string());
    args
}

pub(crate) fn add_offset(c: &mut Command, shift_ms: i64) {
    c.args(offset_args(shift_ms));
}

fn offset_args(shift_ms: i64) -> Vec<OsString> {
    if shift_ms > 0 {
        vec![
            "-itsoffset".into(),
            format!("{:.3}", shift_ms as f64 / 1000.0).into(),
        ]
    } else if shift_ms < 0 {
        vec![
            "-ss".into(),
            format!("{:.3}", (-shift_ms) as f64 / 1000.0).into(),
        ]
    } else {
        vec![]
    }
}

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
