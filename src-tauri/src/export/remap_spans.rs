use super::TimeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipSpan {
    pub clip: usize,
    pub plan_start: usize,
    pub plan_len: usize,
    pub first_k: u64,
}

impl TimeMap {
    pub fn clip_spans(&self, fps: u64) -> Vec<ClipSpan> {
        let mut out: Vec<ClipSpan> = Vec::new();
        let mut at = 0usize;
        for (i, s) in self.segments().iter().enumerate() {
            let Some((k_start, k_end)) = self.frame_bounds(i, fps) else {
                continue;
            };
            let n = ((k_end - k_start) as f64 / s.factor).floor() as usize + 1;
            match out.last_mut() {
                Some(last) if last.clip == s.clip => last.plan_len += n,
                _ => out.push(ClipSpan {
                    clip: s.clip,
                    plan_start: at,
                    plan_len: n,
                    first_k: k_start,
                }),
            }
            at += n;
        }
        out
    }
}

#[cfg(test)]
#[path = "remap_spans_tests.rs"]
mod tests;
