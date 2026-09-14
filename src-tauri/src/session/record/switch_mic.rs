//! Mid-take microphone switching (2026-09-14 design: `docs/superpowers/specs/
//! 2026-09-14-mid-take-source-switching-design.md`). A `cpal` stream is `!Send` and owns one WAV
//! for its whole life, so changing device mid-take means ending the current mic thread and
//! starting another on a NEW file: `mic_2.wav`, `mic_3.wav`... Each one is a `Segment` in the
//! take's `SegmentLog`, stamped with the switch instant on the recording clock, and
//! `export::preview::segments_audio::merge_mic_segments` folds them all back into the single
//! `mic.wav` before the editor opens - so nothing downstream ever learns a switch happened.
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::session::record::emit::{emitter, level_emitter};
use crate::session::record::recorder::Recorder;
use crate::session::record::recorder_threads::spawn_mic_thread;
use crate::session::record::segments;
use crate::session::sync::Segment;

/// Switch the running take's microphone to `device_id` (`None` = mic off from here on).
#[tauri::command]
pub fn switch_mic(
    device_id: Option<String>,
    recorder: tauri::State<'_, Recorder>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // The whole switch runs under the recorder's own lock: a second `switch_mic`, or a Stop, must
    // never see the take between "the old thread is gone" and "the new one owns `mic_stop`" - that
    // window is where a take could end up with two live mic streams, or with none and no segment.
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    let r = guard.as_mut().ok_or("not recording")?;

    // Its OWN flag, never the take-wide `stop`: the system-audio thread and the video pipeline
    // keep running. Joining is what finalizes the current WAV (hound rewrites the RIFF/data sizes
    // in `CpalMicHandle::stop`), so the file is complete and probeable before the next one opens.
    r.mic_stop.store(true, Ordering::SeqCst);
    if let Some(t) = r.mic_thread.take() { let _ = t.join(); }

    // Stamped AFTER the join, so the recording-clock instant is where the new segment really
    // begins rather than where the request arrived.
    let start_ms = segments::recording_ms(r.clock.as_ref(), &r.paused_totals);
    let name = {
        let log = r.segments.lock().unwrap_or_else(|e| e.into_inner());
        segments::next_name("mic", "wav", log.mic.len())
    };
    let path = Path::new(&r.folder).join(&name);

    let stop = Arc::new(AtomicBool::new(false));
    r.mic_stop = stop.clone();
    // A fresh first-sample stamp per segment, deliberately NOT `Running.mic_start`: that one is
    // `sync.json`'s `mic_ms`, which places the FIRST segment, and the merge measures every later
    // segment from it. A later segment's own stamp has no reader, so it is dropped here.
    let started = Arc::new(AtomicU64::new(0));
    // `level_emitter(&app, "mic")` exactly as `start_recording` builds it, so the HUD's wave
    // meter keeps reading the capture that is actually writing the WAV.
    r.mic_thread = spawn_mic_thread(
        device_id, path.to_string_lossy().into_owned(), stop, r.paused.clone(),
        r.clock.clone(), started, emitter(&app, "record-warning"), Some(level_emitter(&app, "mic")),
    );
    // Logged even when the mic was switched OFF (no thread, no file): the boundary still numbers
    // the next segment correctly, and the merge simply finds nothing at that path and skips it.
    r.segments.lock().unwrap_or_else(|e| e.into_inner()).mic.push(Segment { path: name, start_ms });
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::session::record::segments::{next_name, shared};
    use crate::session::sync::Segment;

    /// The naming the command drives: the log holds only the EXTRA segments, so the n-th switch
    /// opens `mic_<n+1>.wav`. A switch to "no mic" still occupies its number.
    #[test]
    fn each_switch_opens_the_next_numbered_wav() {
        let log = shared();
        for expected in ["mic_2.wav", "mic_3.wav", "mic_4.wav"] {
            let mut l = log.lock().unwrap();
            let name = next_name("mic", "wav", l.mic.len());
            assert_eq!(name, expected);
            l.mic.push(Segment { path: name, start_ms: 0 });
        }
        assert_eq!(log.lock().unwrap().mic.len(), 3);
    }
}
