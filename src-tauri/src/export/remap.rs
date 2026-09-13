// The one clip-to-output clock map: trim, cuts and speed spans become kept segments of CLIP time
// (0 = the first video frame), each with a factor and the OUTPUT time it starts at. Everything the
// viewer sees runs on the output clock (regions are moved there by `edit::remap_doc`); sources are
// sampled on the clip clock through `frame_plan`. Mirrored line for line by `src/lib/remap.ts` with
// the same f64 expression order, so both sides agree to the bit. Design and the fingerprints:
// docs/superpowers/specs/2026-09-13-time-remap-design.md.
use crate::edit::model::{Cut, Speed, Trim};

pub const FACTOR_MIN: f64 = 0.25;
pub const FACTOR_MAX: f64 = 8.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Segment { pub clip_start: u32, pub clip_end: u32, pub factor: f64, pub out_start: f64 }

#[derive(Clone, Debug)]
pub struct TimeMap { segs: Vec<Segment>, out_dur: f64, trim_in: u32, plain: bool }

/// Cuts clamped into `[lo, hi]`, sorted, with overlapping or touching ones merged into one.
fn merged_cuts(cuts: &[Cut], lo: u32, hi: u32) -> Vec<(u32, u32)> {
    let mut v: Vec<(u32, u32)> = cuts.iter().map(|c| (c.start_ms.clamp(lo, hi), c.end_ms.clamp(lo, hi))).filter(|(a, b)| a < b).collect();
    v.sort_unstable();
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (a, b) in v {
        match out.last_mut() { Some(last) if a <= last.1 => last.1 = last.1.max(b), _ => out.push((a, b)) }
    }
    out
}

/// Speed spans clamped into `[lo, hi]`, sorted by start, a later span clamped to start at its
/// predecessor's end, factors clamped to the published range, empty spans dropped.
fn clamped_spans(speed: &[Speed], lo: u32, hi: u32) -> Vec<(u32, u32, f64)> {
    let mut v: Vec<(u32, u32, f64)> = speed.iter()
        .map(|s| (s.start_ms.clamp(lo, hi), s.end_ms.clamp(lo, hi), (s.factor as f64).clamp(FACTOR_MIN, FACTOR_MAX))).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out: Vec<(u32, u32, f64)> = Vec::new();
    for (a, b, f) in v {
        let a = out.last().map_or(a, |l| a.max(l.1));
        if a < b { out.push((a, b, f)); }
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
        for (a, b) in &cuts { if at < *a { kept.push((at, *a)); } at = at.max(*b); }
        if at < hi { kept.push((at, hi)); }
        let mut segs: Vec<Segment> = Vec::new();
        let mut out_start = 0.0f64;
        for (ks, ke) in kept {
            let mut edges: Vec<u32> = vec![ks, ke];
            for (a, b, _) in &spans { for e in [a, b] { if *e > ks && *e < ke { edges.push(*e); } } }
            edges.sort_unstable(); edges.dedup();
            for w in edges.windows(2) {
                let (cs, ce) = (w[0], w[1]);
                let factor = spans.iter().find(|(a, b, _)| *a <= cs && ce <= *b).map_or(1.0, |s| s.2);
                segs.push(Segment { clip_start: cs, clip_end: ce, factor, out_start });
                out_start += (ce - cs) as f64 / factor;
            }
        }
        TimeMap { segs, out_dur: out_start, trim_in: lo, plain: cuts.is_empty() && spans.is_empty() }
    }

    pub fn identity(full_dur_ms: u32) -> TimeMap { TimeMap::build(&Trim::default(), &[], &[], full_dur_ms) }
    /// No cuts and no speed spans (a trim may still exist): the plan is `k_in ..= k_last` and every
    /// evaluator sees the clip clock, so nothing downstream changes.
    pub fn is_plain(&self) -> bool { self.plain }
    pub fn segments(&self) -> &[Segment] { &self.segs }
    pub fn out_dur_ms(&self) -> u32 { self.out_dur.round() as u32 }

    /// Output time of a clip time: piecewise linear inside a segment, the NEXT segment's start
    /// inside a gap (the frame the viewer sees next), the total past the last segment.
    pub fn out_of(&self, clip_ms: u32) -> u32 {
        for s in &self.segs {
            if clip_ms < s.clip_start { return s.out_start.round() as u32; }
            if clip_ms < s.clip_end { return (s.out_start + (clip_ms - s.clip_start) as f64 / s.factor).round() as u32; }
        }
        self.out_dur.round() as u32
    }

    /// Clip time of an output time on the kept ranges; at or past the end, the last kept edge.
    pub fn clip_of(&self, out_ms: u32) -> u32 {
        let o = out_ms as f64;
        for s in &self.segs {
            let len = (s.clip_end - s.clip_start) as f64 / s.factor;
            if o < s.out_start + len { return (s.clip_start as f64 + (o - s.out_start) * s.factor).round() as u32; }
        }
        self.segs.last().map_or(self.trim_in, |s| s.clip_end)
    }

    /// The gap (a cut, or the trim's outside) a clip time falls in: `(previous kept end, next kept
    /// start)`, the trailing gap running to `u32::MAX`. `None` on a kept range.
    pub fn gap_containing(&self, clip_ms: u32) -> Option<(u32, u32)> {
        let mut prev_end = 0u32;
        for s in &self.segs {
            if clip_ms < s.clip_start { return Some((prev_end, s.clip_start)); }
            if clip_ms < s.clip_end { return None; }
            prev_end = s.clip_end;
        }
        Some((prev_end, u32::MAX))
    }

    /// Whether the recording frames skipped between two consecutive plan entries include a cut
    /// (the frame right after `prev_k` lies in a gap). A speed-span skip has no gap, so it is false.
    pub fn crosses_cut(&self, prev_k: u64, k: u64, fps: u64) -> bool {
        k > prev_k + 1 && self.gap_containing(((prev_k + 1) * 1000 / fps) as u32).is_some()
    }

    /// Inclusive recording-frame bounds of segment `i`. The FIRST segment floors its start and the
    /// LAST floors its end (today's trim semantics, which keeps the trim-only plan bit-identical);
    /// a cut edge is exact: the frames whose time lies inside the cut are the ones removed.
    pub fn frame_bounds(&self, i: usize, fps: u64) -> Option<(u64, u64)> {
        let s = self.segs.get(i)?;
        let (cs, ce) = (s.clip_start as u64, s.clip_end as u64);
        let k_start = if i == 0 { cs * fps / 1000 } else { (cs * fps + 999) / 1000 };
        let k_end = if i + 1 == self.segs.len() { ce * fps / 1000 } else { ((ce * fps + 999) / 1000).checked_sub(1)? };
        (k_end >= k_start).then_some((k_start, k_end))
    }

    /// For every output frame, the recording frame it shows: a factor above 1 skips frames, one
    /// below 1 repeats them. Monotone non-decreasing by construction.
    pub fn frame_plan(&self, fps: u64) -> Vec<u64> {
        let mut plan = Vec::new();
        for (i, s) in self.segs.iter().enumerate() {
            let Some((k_start, k_end)) = self.frame_bounds(i, fps) else { continue };
            let n = ((k_end - k_start) as f64 / s.factor).floor() as u64 + 1;
            plan.extend((0..n).map(|j| k_start + (j as f64 * s.factor).floor() as u64));
        }
        plan
    }
}

#[cfg(test)]
#[path = "remap_tests.rs"]
pub(crate) mod tests;
