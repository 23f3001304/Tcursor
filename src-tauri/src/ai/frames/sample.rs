pub const MAX_FRAMES: usize = 16;

pub const IDLE_EVERY_MS: u32 = 8_000;

const MIN_GAP_MS: u32 = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameReason {
    Click,
    Layout,
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameAt {
    pub t_ms: u32,
    pub reason: FrameReason,
}

fn rank(r: FrameReason) -> u8 {
    match r {
        FrameReason::Click => 0,
        FrameReason::Layout => 1,
        FrameReason::Idle => 2,
    }
}

pub fn sample_times(clicks: &[u32], layouts: &[u32], dur_ms: u32, cap: usize) -> Vec<FrameAt> {
    let mut all: Vec<FrameAt> = clicks
        .iter()
        .map(|&t_ms| FrameAt {
            t_ms,
            reason: FrameReason::Click,
        })
        .chain(layouts.iter().map(|&t_ms| FrameAt {
            t_ms,
            reason: FrameReason::Layout,
        }))
        .filter(|f| f.t_ms < dur_ms)
        .collect();
    let guard = IDLE_EVERY_MS / 2;
    let idle: Vec<FrameAt> = (0..dur_ms)
        .step_by(IDLE_EVERY_MS as usize)
        .filter(|&t| !all.iter().any(|r| t.abs_diff(r.t_ms) < guard))
        .map(|t_ms| FrameAt {
            t_ms,
            reason: FrameReason::Idle,
        })
        .collect();
    all.extend(idle);

    all.sort_by(|a, b| {
        a.t_ms
            .cmp(&b.t_ms)
            .then(rank(a.reason).cmp(&rank(b.reason)))
    });
    let mut kept: Vec<FrameAt> = Vec::new();
    for f in all {
        if kept.last().is_some_and(|k| f.t_ms - k.t_ms < MIN_GAP_MS) {
            continue;
        }
        kept.push(f);
    }
    if kept.len() > cap {
        kept = thin(kept, cap);
    }
    kept.sort_by_key(|f| f.t_ms);
    kept
}

fn thin(all: Vec<FrameAt>, cap: usize) -> Vec<FrameAt> {
    let (real, idle): (Vec<FrameAt>, Vec<FrameAt>) =
        all.into_iter().partition(|f| f.reason != FrameReason::Idle);
    let keep_real = real.len().min(cap);
    let mut out = spread(&real, keep_real);
    out.extend(spread(&idle, cap - keep_real));
    out
}

fn spread(src: &[FrameAt], keep: usize) -> Vec<FrameAt> {
    if keep == 0 || src.is_empty() {
        return Vec::new();
    }
    if keep >= src.len() {
        return src.to_vec();
    }
    if keep == 1 {
        return vec![src[0]];
    }
    (0..keep)
        .map(|j| src[j * (src.len() - 1) / (keep - 1)])
        .collect()
}

#[cfg(test)]
#[path = "sample_tests.rs"]
mod tests;
