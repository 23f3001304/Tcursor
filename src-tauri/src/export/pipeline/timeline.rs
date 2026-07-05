use crate::events::model::EventLog;
use crate::export::pipeline::ffio::{probe_duration, probe_frame_count};
use crate::session::paths::ProjectPaths;
use crate::session::sync::SyncLog;

/// The real capture timeline: the clock time (ms) of each encoded video frame,
/// plus the audio/event start times, so the export can place every frame by its
/// true timestamp (fps-agnostic) and align audio to the video start.
pub struct Timeline {
    pub frames: Vec<u64>,
    pub events_ms: u64,
    pub mic_ms: Option<u64>,
    pub system_ms: Option<u64>,
}

/// Use the recorded `sync.json` when present; otherwise synthesize a uniform
/// timeline (frame count spread over the real audio/event duration) so older
/// recordings still export correctly through the same timestamp path.
pub fn build_timeline(paths: &ProjectPaths, log: &EventLog, fps: u32) -> Timeline {
    if let Ok(s) = SyncLog::load(&paths.sync()) {
        if !s.frames.is_empty() {
            return Timeline { frames: s.frames, events_ms: s.events_ms, mic_ms: s.mic_ms, system_ms: s.system_ms };
        }
    }
    let video = paths.video();
    let count = probe_frame_count(&video).unwrap_or(0).max(1);
    let dur = probe_duration(&video).unwrap_or(0.0);
    let ref_dur = audio_dur(paths)
        .or_else(|| log.events.last().map(|e| e.t as f64 / 1000.0))
        .filter(|d| *d > 0.1)
        .unwrap_or(if dur > 0.1 { dur } else { count as f64 / fps.max(1) as f64 });
    let efps = (count as f64 / ref_dur).max(1.0);
    let frames = (0..count).map(|i| (i as f64 * 1000.0 / efps).round() as u64).collect();
    Timeline {
        frames,
        events_ms: 0,
        mic_ms: paths.mic().exists().then_some(0),
        system_ms: paths.system().exists().then_some(0),
    }
}

/// Duration (s) of whichever audio will be muxed (mic preferred), for synthesis.
fn audio_dur(paths: &ProjectPaths) -> Option<f64> {
    for p in [paths.mic(), paths.system()] {
        if p.exists() {
            if let Ok(d) = probe_duration(&p) {
                if d > 0.1 { return Some(d); }
            }
        }
    }
    None
}
