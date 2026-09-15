use std::time::Instant;

pub(super) fn tick_progress(
    done: u64,
    total_out: u64,
    last_pct: u8,
    start: Instant,
    on_progress: &impl Fn(u8),
) -> u8 {
    let pct = ((done * 100 / total_out.max(1)).min(100)) as u8;
    if pct == last_pct {
        return last_pct;
    }
    on_progress(pct);
    if pct % 5 == 0 {
        let elapsed = start.elapsed().as_secs_f64();
        eprintln!(
            "[EXPORT] Progress: {:3}% | frame {:5}/{} | speed: {:.1} FPS | elapsed: {:.1}s",
            pct,
            done,
            total_out,
            done as f64 / elapsed.max(0.001),
            elapsed
        );
    }
    pct
}

pub(super) fn log_timing(total_out: u64, secs: f64, t_dec: u128, t_comp: u128, t_send: u128) {
    eprintln!(
        "[EXPORT] Finished render: {} frames in {:.2}s ({:.1} FPS)",
        total_out,
        secs,
        total_out as f64 / secs
    );
    let _ = std::fs::write(
        std::env::temp_dir().join("tcursor-export-timing.txt"),
        format!(
            "frames={} total={:.2}s fps={:.1} decode={}ms composite={}ms encode_wait={}ms\n",
            total_out,
            secs,
            total_out as f64 / secs,
            t_dec / 1000,
            t_comp / 1000,
            t_send / 1000
        ),
    );
}
