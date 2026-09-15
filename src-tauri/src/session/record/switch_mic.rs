use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::session::record::emit::{emitter, level_emitter};
use crate::session::record::recorder::Recorder;
use crate::session::record::recorder_threads::spawn_mic_thread;
use crate::session::record::segments;
use crate::session::sync::Segment;

#[tauri::command]
pub fn switch_mic(
    device_id: Option<String>,
    recorder: tauri::State<'_, Recorder>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = recorder.inner.lock().unwrap_or_else(|e| e.into_inner());
    let r = guard.as_mut().ok_or("not recording")?;

    r.mic_stop.store(true, Ordering::SeqCst);
    if let Some(t) = r.mic_thread.take() {
        let _ = t.join();
    }

    let start_ms = segments::recording_ms(r.clock.as_ref(), &r.paused_totals);
    let name = {
        let log = r.segments.lock().unwrap_or_else(|e| e.into_inner());
        segments::next_name("mic", "wav", log.mic.len())
    };
    let path = Path::new(&r.folder).join(&name);

    let stop = Arc::new(AtomicBool::new(false));
    r.mic_stop = stop.clone();
    let started = Arc::new(AtomicU64::new(0));
    r.mic_thread = spawn_mic_thread(
        device_id,
        path.to_string_lossy().into_owned(),
        stop,
        r.paused.clone(),
        r.clock.clone(),
        started,
        emitter(&app, "record-warning"),
        Some(level_emitter(&app, "mic")),
    );
    r.segments
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .mic
        .push(Segment {
            path: name,
            start_ms,
        });
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::session::record::segments::{next_name, shared};
    use crate::session::sync::Segment;

    #[test]
    fn each_switch_opens_the_next_numbered_wav() {
        let log = shared();
        for expected in ["mic_2.wav", "mic_3.wav", "mic_4.wav"] {
            let mut l = log.lock().unwrap();
            let name = next_name("mic", "wav", l.mic.len());
            assert_eq!(name, expected);
            l.mic.push(Segment {
                path: name,
                start_ms: 0,
            });
        }
        assert_eq!(log.lock().unwrap().mic.len(), 3);
    }
}
