use crate::edit::model::{Cut, Speed, Trim};

pub const FACTOR_MIN: f64 = 0.25;
pub const FACTOR_MAX: f64 = 8.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub clip_start: u32,
    pub clip_end: u32,
    pub factor: f64,
    pub out_start: f64,
}

#[derive(Clone, Debug)]
pub struct TimeMap {
    segs: Vec<Segment>,
    out_dur: f64,
    trim_in: u32,
    plain: bool,
}

fn merged_cuts(cuts: &[Cut], lo: u32, hi: u32) -> Vec<(u32, u32)> {
    let mut v: Vec<(u32, u32)> = cuts
        .iter()
        .map(|c| (c.start_ms.clamp(lo, hi), c.end_ms.clamp(lo, hi)))
        .filter(|(a, b)| a < b)
        .collect();
    v.sort_unstable();
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (a, b) in v {
        match out.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

fn clamped_spans(speed: &[Speed], lo: u32, hi: u32) -> Vec<(u32, u32, f64)> {
    let mut v: Vec<(u32, u32, f64)> = speed
        .iter()
        .map(|s| {
            (
                s.start_ms.clamp(lo, hi),
                s.end_ms.clamp(lo, hi),
                (s.factor as f64).clamp(FACTOR_MIN, FACTOR_MAX),
            )
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out: Vec<(u32, u32, f64)> = Vec::new();
    for (a, b, f) in v {
        let a = out.last().map_or(a, |l| a.max(l.1));
        if a < b {
            out.push((a, b, f));
        }
    }
    out
}

impl TimeMap {
    pub fn build(trim: &Trim, cuts: &[Cut], speed: &[Speed], full_dur_ms: u32) -> TimeMap {
        let (lo, hi) = trim.resolve(full_dur_ms);
        let cuts = merged_cuts(cuts, lo, hi);
        let spans = clamped_spans(speed, lo, hi);
        let mut kept: Vec<(u32, u32)> = Vec::new();
        let mut at = lo;
        for (a, b) in &cuts {
            if at < *a {
                kept.push((at, *a));
            }
            at = at.max(*b);
        }
        if at < hi {
            kept.push((at, hi));
        }
        let mut segs: Vec<Segment> = Vec::new();
        let mut out_start = 0.0f64;
        for (keep_start, keep_end) in kept {
            let mut edges: Vec<u32> = vec![keep_start, keep_end];
            for (a, b, _) in &spans {
                for e in [a, b] {
                    if *e > keep_start && *e < keep_end {
                        edges.push(*e);
                    }
                }
            }
            edges.sort_unstable();
            edges.dedup();
            for w in edges.windows(2) {
                let (seg_start, seg_end) = (w[0], w[1]);
                let factor = spans
                    .iter()
                    .find(|(a, b, _)| *a <= seg_start && seg_end <= *b)
                    .map_or(1.0, |s| s.2);
                segs.push(Segment {
                    clip_start: seg_start,
                    clip_end: seg_end,
                    factor,
                    out_start,
                });
                out_start += (seg_end - seg_start) as f64 / factor;
            }
        }
        TimeMap {
            segs,
            out_dur: out_start,
            trim_in: lo,
            plain: cuts.is_empty() && spans.is_empty(),
        }
    }

    pub fn identity(full_dur_ms: u32) -> TimeMap {
        TimeMap::build(&Trim::default(), &[], &[], full_dur_ms)
    }
    pub fn is_plain(&self) -> bool {
        self.plain
    }
    pub fn segments(&self) -> &[Segment] {
        &self.segs
    }
    pub fn out_dur_ms(&self) -> u32 {
        self.out_dur.round() as u32
    }

    pub fn out_of(&self, clip_ms: u32) -> u32 {
        for s in &self.segs {
            if clip_ms < s.clip_start {
                return s.out_start.round() as u32;
            }
            if clip_ms < s.clip_end {
                return (s.out_start + (clip_ms - s.clip_start) as f64 / s.factor).round() as u32;
            }
        }
        self.out_dur.round() as u32
    }

    pub fn clip_of(&self, out_ms: u32) -> u32 {
        let o = out_ms as f64;
        for s in &self.segs {
            let len = (s.clip_end - s.clip_start) as f64 / s.factor;
            if o < s.out_start + len {
                return (s.clip_start as f64 + (o - s.out_start) * s.factor).round() as u32;
            }
        }
        self.segs.last().map_or(self.trim_in, |s| s.clip_end)
    }

    pub fn gap_containing(&self, clip_ms: u32) -> Option<(u32, u32)> {
        let mut prev_end = 0u32;
        for s in &self.segs {
            if clip_ms < s.clip_start {
                return Some((prev_end, s.clip_start));
            }
            if clip_ms < s.clip_end {
                return None;
            }
            prev_end = s.clip_end;
        }
        Some((prev_end, u32::MAX))
    }

    pub fn crosses_cut(&self, prev_k: u64, k: u64, fps: u64) -> bool {
        k > prev_k + 1
            && self
                .gap_containing(((prev_k + 1) * 1000 / fps) as u32)
                .is_some()
    }

    pub fn frame_bounds(&self, i: usize, fps: u64) -> Option<(u64, u64)> {
        let s = self.segs.get(i)?;
        let (seg_start, seg_end) = (s.clip_start as u64, s.clip_end as u64);
        let k_start = if i == 0 {
            seg_start * fps / 1000
        } else {
            (seg_start * fps + 999) / 1000
        };
        let k_end = if i + 1 == self.segs.len() {
            seg_end * fps / 1000
        } else {
            ((seg_end * fps + 999) / 1000).checked_sub(1)?
        };
        (k_end >= k_start).then_some((k_start, k_end))
    }

    pub fn frame_plan(&self, fps: u64) -> Vec<u64> {
        let mut plan = Vec::new();
        for (i, s) in self.segs.iter().enumerate() {
            let Some((k_start, k_end)) = self.frame_bounds(i, fps) else {
                continue;
            };
            let n = ((k_end - k_start) as f64 / s.factor).floor() as u64 + 1;
            plan.extend((0..n).map(|j| k_start + (j as f64 * s.factor).floor() as u64));
        }
        plan
    }
}

#[cfg(test)]
#[path = "remap_tests.rs"]
pub(crate) mod tests;
