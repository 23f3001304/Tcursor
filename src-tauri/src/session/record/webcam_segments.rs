//! The camera half of mid-take source switching (2026-09-14): which file a webcam chunk appends
//! to, and the command the HUD calls at the instant it swaps cameras. A `MediaRecorder` cannot
//! change its stream, so a switch is a SECOND recorder writing a second file; this module names
//! that file and stamps where it starts on the recording clock, and
//! `export::preview::segments_webcam` merges the lot back into one `webcam.webm` at Stop.
use crate::session::record::recorder::Recorder;
use crate::session::record::segments::recording_ms;
use crate::session::sync::Segment;

/// The file a webcam chunk with this segment index belongs in. `None` (a HUD that predates
/// switching), `0` and `1` are all the take's first segment, which keeps the plain `webcam.webm`
/// name every reader downstream already knows.
pub fn webcam_segment_name(segment: Option<u32>) -> String {
    match segment {
        None | Some(0) | Some(1) => "webcam.webm".to_string(),
        Some(n) => format!("webcam_{n}.webm"),
    }
}

/// Record that the take's camera just changed: the HUD calls this after the previous recorder has
/// flushed and the new stream is live, immediately before `append_webcam` starts writing
/// `webcam_<n>.webm`. Pushing the switch instant (on the recording clock, so a paused span never
/// counts) is all the merge needs to place the new segment.
#[tauri::command]
pub fn mark_webcam_segment(segment: u32, recorder: tauri::State<'_, Recorder>) -> Result<(), String> {
    if segment < 2 { return Err("the first webcam segment is webcam.webm and is never marked".into()); }
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    let r = guard.as_ref().ok_or("not recording")?;
    let start_ms = recording_ms(r.clock.as_ref(), &r.paused_totals);
    let mut log = r.segments.lock().unwrap_or_else(|e| e.into_inner());
    log.webcam.push(Segment { path: webcam_segment_name(Some(segment)), start_ms });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_segment_keeps_the_name_every_reader_knows() {
        assert_eq!(webcam_segment_name(None), "webcam.webm");
        assert_eq!(webcam_segment_name(Some(0)), "webcam.webm");
        assert_eq!(webcam_segment_name(Some(1)), "webcam.webm");
    }

    #[test]
    fn every_later_segment_is_numbered_from_two() {
        assert_eq!(webcam_segment_name(Some(2)), "webcam_2.webm");
        assert_eq!(webcam_segment_name(Some(11)), "webcam_11.webm");
    }
}
