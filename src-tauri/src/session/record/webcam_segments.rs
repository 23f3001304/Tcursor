use crate::session::record::recorder::Recorder;
use crate::session::record::segments::recording_ms;
use crate::session::sync::Segment;

pub fn webcam_segment_name(segment: Option<u32>) -> String {
    match segment {
        None | Some(0) | Some(1) => "webcam.webm".to_string(),
        Some(n) => format!("webcam_{n}.webm"),
    }
}

#[tauri::command]
pub fn mark_webcam_segment(
    segment: u32,
    recorder: tauri::State<'_, Recorder>,
) -> Result<(), String> {
    if segment < 2 {
        return Err("the first webcam segment is webcam.webm and is never marked".into());
    }
    let guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    let r = guard.as_ref().ok_or("not recording")?;
    let start_ms = recording_ms(r.clock.as_ref(), &r.paused_totals);
    let mut log = r.segments.lock().unwrap_or_else(|e| e.into_inner());
    log.webcam.push(Segment {
        path: webcam_segment_name(Some(segment)),
        start_ms,
    });
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
