// What the export SAYS about itself while it runs: the progress callback's rate-limiting, the
// periodic speed line, and the per-stage timing breakdown written at the end. Split out of
// `exporter.rs` for its line budget; pure reporting, no effect on a single pixel.
use std::time::Instant;

/// Report one completed frame. Returns the percentage now showing, which the caller keeps as
/// `last_pct` - `on_progress` fires only when the whole number CHANGES, so a 30-minute export
/// calls into the frontend 100 times rather than 100,000.
///
/// `done` counts completed frames (1-based), so it - unlike the raw index `k - k_in` - reaches
/// exactly `total_out` on the last frame actually sent, i.e. 100%.
pub(super) fn tick_progress(done: u64, total_out: u64, last_pct: u8, start: Instant,
                            on_progress: &impl Fn(u8)) -> u8 {
    let pct = ((done * 100 / total_out.max(1)).min(100)) as u8;
    if pct == last_pct { return last_pct; }
    on_progress(pct);
    // Every 5% (not every 1%) for the log: enough to watch a long export's pace without turning
    // stderr into the export's own bottleneck.
    if pct % 5 == 0 {
        let elapsed = start.elapsed().as_secs_f64();
        eprintln!("[EXPORT] Progress: {:3}% | frame {:5}/{} | speed: {:.1} FPS | elapsed: {:.1}s",
            pct, done, total_out, done as f64 / elapsed.max(0.001), elapsed);
    }
    pct
}

/// The finishing lines plus `%TEMP%/tcursor-export-timing.txt`. The three stage totals are
/// microseconds spent BLOCKED in each stage: with the pipeline overlapping, the total trends
/// toward `max(decode, composite)` rather than their sum, so a lopsided pair is the useful signal.
pub(super) fn log_timing(total_out: u64, secs: f64, t_dec: u128, t_comp: u128, t_send: u128) {
    eprintln!("[EXPORT] Finished render: {} frames in {:.2}s ({:.1} FPS)", total_out, secs, total_out as f64 / secs);
    let _ = std::fs::write(std::env::temp_dir().join("tcursor-export-timing.txt"), format!(
        "frames={} total={:.2}s fps={:.1} decode={}ms composite={}ms encode_wait={}ms\n",
        total_out, secs, total_out as f64 / secs, t_dec / 1000, t_comp / 1000, t_send / 1000));
}
