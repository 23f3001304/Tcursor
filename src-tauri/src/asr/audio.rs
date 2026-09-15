use crate::edit::captions::CaptionWord;
use crate::session::paths::ProjectPaths;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsrSource {
    Mic,
    System,
}

pub fn pick_source(paths: &ProjectPaths) -> Option<AsrSource> {
    if paths.mic().exists() {
        Some(AsrSource::Mic)
    } else if paths.system().exists() {
        Some(AsrSource::System)
    } else {
        None
    }
}

pub fn wav_for(paths: &ProjectPaths, src: AsrSource) -> PathBuf {
    match src {
        AsrSource::Mic => paths.mic(),
        AsrSource::System => paths.system(),
    }
}

pub fn to_output_ms(audio_ms: i64, shift_ms: i64) -> Option<u32> {
    let t = audio_ms + shift_ms;
    if t < 0 {
        None
    } else {
        Some(t as u32)
    }
}

pub fn shift_words(words: Vec<CaptionWord>, shift_ms: i64) -> Vec<CaptionWord> {
    words
        .into_iter()
        .filter_map(|w| {
            let end = to_output_ms(w.end_ms as i64, shift_ms)?;
            let start = to_output_ms(w.start_ms as i64, shift_ms).unwrap_or(0);
            (end > start).then_some(CaptionWord {
                start_ms: start,
                end_ms: end,
                text: w.text,
            })
        })
        .collect()
}

pub fn source_shift_ms(paths: &ProjectPaths, src: AsrSource) -> i64 {
    let Ok(log) = crate::events::model::EventLog::load(&paths.events()) else {
        return 0;
    };
    let tl = crate::export::pipeline::timeline::build_timeline(paths, &log, 60);
    let track_ms = match src {
        AsrSource::Mic => tl.mic_ms,
        AsrSource::System => tl.system_ms,
    };
    crate::export::pipeline::audio_shift_ms(track_ms, tl.frames.first().copied().unwrap_or(0), 0)
}

pub fn decode_16k_mono(wav: &Path) -> Result<Vec<f32>, String> {
    let out = crate::process::proc::ffcmd("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(wav)
        .args(["-f", "f32le", "-ac", "1", "-ar", "16000", "-"])
        .output()
        .map_err(|e| format!("could not run ffmpeg to decode the audio: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg could not decode {}: {}",
            wav.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    if out.stdout.is_empty() {
        return Err("this recording's audio track is empty.".into());
    }
    if out.stdout.len() % 4 != 0 {
        return Err("the decoded audio ended mid-sample.".into());
    }
    Ok(out
        .stdout
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
