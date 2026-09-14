//! The cursor's OFFLINE path model. The recorded route is split into RESTS - where the hand held
//! still, which is also where every click happens - and the MOVES between them. Polish (smoothness,
//! idealization) is applied to the moves only, with both ends pinned, so the drawn cursor is exactly
//! where the real one was whenever it rested or clicked; only the glide between those points is
//! idealized. Nothing here lags: a move arrives when the recording arrived.
use crate::events::model::{EventKind, MouseEvent, ScreenInfo};
use crate::export::coordmap::to_frame;

/// How long the hand must hold still (within `rest_px`) for a pause to count as a rest.
pub const REST_MS: u32 = 120;
/// A move's first vertex leaves its rest at most this long before the first sample that left.
const LEAD_MS: u32 = 16;
/// Stroke resampling: about one point per this many px of raw travel, within `[MIN_PTS, MAX_PTS]`.
const PX_PER_PT: f32 = 3.0;
const MIN_PTS: usize = 8;
const MAX_PTS: usize = 240;
/// At full smoothness the spatial window reaches this fraction of the stroke on each side.
const SMOOTH_SPAN: f32 = 0.15;

/// The radius, in frame px, a rest may wander within: ~4 px on a 1920-wide capture.
pub fn rest_px(screen_w: u32) -> f32 { (screen_w as f32 / 480.0).max(3.0) }

#[derive(Clone, Copy, Debug)]
struct Pt { t: u32, x: f32, y: f32 }

/// One piece of the route over samples `i0..=i1`: a rest holds (raw) from `t0` until the next move
/// starts; a move is the polyline from one rest's last sample to the next rest's first, timed
/// `[t0, t1]`.
#[derive(Clone, Copy, Debug)]
struct Seg { t0: u32, t1: u32, i0: usize, i1: usize, rest: bool }

pub struct PathModel {
    pts: Vec<Pt>,
    arc: Vec<f32>, // cumulative raw arc length per sample (px)
    segs: Vec<Seg>,
    /// Per segment, a move's polished polyline (uniform in RAW arc length), built lazily for `key`.
    strokes: Vec<Option<Vec<(f32, f32)>>>,
    key: (f32, f32),
}

fn dist(a: Pt, b: Pt) -> f32 { ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt() }
fn smoothstep(f: f32) -> f32 { f * f * (3.0 - 2.0 * f) }
fn lerp(a: (f32, f32), b: (f32, f32), f: f32) -> (f32, f32) { (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f) }

impl PathModel {
    pub fn new(events: &[MouseEvent], screen: &ScreenInfo) -> Self {
        let mut pts: Vec<Pt> = Vec::with_capacity(events.len());
        let mut clicks: Vec<bool> = Vec::with_capacity(events.len());
        for e in events {
            if pts.last().is_some_and(|p| e.t < p.t) { continue; } // the log is chronological; never trust it blindly
            let p = to_frame(screen, e.x, e.y);
            pts.push(Pt { t: e.t, x: p.x as f32, y: p.y as f32 });
            clicks.push(e.kind == EventKind::Down);
        }
        let mut arc = vec![0.0f32; pts.len()];
        for i in 1..pts.len() { arc[i] = arc[i - 1] + dist(pts[i - 1], pts[i]); }
        let segs = segment(&pts, &clicks, rest_px(screen.w));
        let strokes = vec![None; segs.len()];
        Self { pts, arc, segs, strokes, key: (0.0, 0.0) }
    }

    /// The polished position at `t_ms` (frame px), or `None` with no samples at all. `smooth` shapes
    /// the glide (velocity profile + de-jitter), `ideal` straightens the route; both 0..1.
    pub fn at(&mut self, t_ms: u32, smooth: f32, ideal: f32) -> Option<(f32, f32)> {
        let key = (smooth.clamp(0.0, 1.0), ideal.clamp(0.0, 1.0));
        if key != self.key { self.key = key; self.strokes.iter_mut().for_each(|s| *s = None); }
        let si = self.segs.partition_point(|s| s.t0 <= t_ms).checked_sub(1)?;
        let s = self.segs[si];
        if s.rest { return Some(self.rest_at(s, t_ms)); }
        if s.t1 <= s.t0 { let p = self.pts[s.i1]; return Some((p.x, p.y)); }
        let u = ((t_ms.saturating_sub(s.t0)) as f32 / (s.t1 - s.t0) as f32).clamp(0.0, 1.0);
        let raw = self.progress(s, t_ms);
        let f = raw + (smoothstep(u) - raw) * key.0;
        if self.strokes[si].is_none() { self.strokes[si] = Some(self.build_stroke(s, key.0, key.1)); }
        let pts = self.strokes[si].as_ref().unwrap();
        if f >= 1.0 { return Some(pts[pts.len() - 1]); } // the end vertex verbatim, not a lerp that lands an ulp off
        let x = f * (pts.len() - 1) as f32;
        let i = (x.floor() as usize).min(pts.len() - 2);
        Some(lerp(pts[i], pts[i + 1], x - i as f32))
    }

    /// Raw interpolation inside a rest, holding the last sample once it is reached (a hand that
    /// emits no events has not moved - the gap is the rest, not a glide toward the next sample).
    fn rest_at(&self, s: Seg, t_ms: u32) -> (f32, f32) {
        let p = &self.pts[s.i0..=s.i1];
        let k = p.partition_point(|q| q.t <= t_ms);
        if k == 0 { return (p[0].x, p[0].y); }
        if k >= p.len() { let l = p[p.len() - 1]; return (l.x, l.y); }
        let (a, b) = (p[k - 1], p[k]);
        let f = (t_ms - a.t) as f32 / (b.t - a.t).max(1) as f32;
        lerp((a.x, a.y), (b.x, b.y), f)
    }

    /// The raw path's arc-length fraction (0..1) along move `s` at `t_ms`: how far the hand had
    /// really travelled, so a stroke replays the recording's own timing when `smooth` is 0.
    fn progress(&self, s: Seg, t_ms: u32) -> f32 {
        let total = self.arc[s.i1] - self.arc[s.i0];
        if total <= 0.0 { return 1.0; }
        // Vertex 0 is timed at the move's start; the rest keep their own (monotone) sample times.
        let vt = |i: usize| if i == s.i0 { s.t0 } else { self.pts[i].t.max(s.t0) };
        let k = s.i0 + self.pts[s.i0 + 1..=s.i1].partition_point(|q| q.t.max(s.t0) <= t_ms);
        if k >= s.i1 { return 1.0; }
        let (ta, tb) = (vt(k), vt(k + 1));
        let f = if tb > ta { (t_ms.saturating_sub(ta)) as f32 / (tb - ta) as f32 } else { 1.0 };
        let a = self.arc[k] - self.arc[s.i0] + f * (self.arc[k + 1] - self.arc[k]);
        (a / total).clamp(0.0, 1.0)
    }

    /// Resample the move's raw polyline uniformly by arc length, de-jitter it with an
    /// endpoint-preserving moving average (two passes, a window that shrinks to nothing at the
    /// ends), then pull it toward the straight chord. Both ends stay exactly the raw endpoints.
    fn build_stroke(&self, s: Seg, smooth: f32, ideal: f32) -> Vec<(f32, f32)> {
        let (p0, p1) = (self.pts[s.i0], self.pts[s.i1]);
        let total = self.arc[s.i1] - self.arc[s.i0];
        let n = ((total / PX_PER_PT).round() as usize).clamp(MIN_PTS, MAX_PTS) + 1;
        let mut out: Vec<(f32, f32)> = Vec::with_capacity(n);
        let mut k = s.i0;
        for i in 0..n {
            let target = self.arc[s.i0] + total * i as f32 / (n - 1) as f32;
            while k + 1 < s.i1 && self.arc[k + 1] < target { k += 1; }
            let (a, b) = (self.pts[k], self.pts[(k + 1).min(s.i1)]);
            let span = self.arc[(k + 1).min(s.i1)] - self.arc[k];
            let f = if span > 0.0 { ((target - self.arc[k]) / span).clamp(0.0, 1.0) } else { 0.0 };
            out.push(lerp((a.x, a.y), (b.x, b.y), f));
        }
        out[0] = (p0.x, p0.y); out[n - 1] = (p1.x, p1.y);
        let w = (smooth * SMOOTH_SPAN * n as f32).round() as usize;
        for _ in 0..2 { if w > 0 { out = pinned_average(&out, w); } }
        for (i, p) in out.iter_mut().enumerate() {
            let chord = lerp((p0.x, p0.y), (p1.x, p1.y), i as f32 / (n - 1) as f32);
            *p = lerp(*p, chord, ideal);
        }
        out
    }
}

/// Moving average with half-window `min(w, i, n-1-i)`: full strength mid-stroke, exactly the
/// input at both ends. Prefix sums keep it O(n).
fn pinned_average(p: &[(f32, f32)], w: usize) -> Vec<(f32, f32)> {
    let n = p.len();
    let mut sx = vec![0.0f32; n + 1]; let mut sy = vec![0.0f32; n + 1];
    for i in 0..n { sx[i + 1] = sx[i] + p[i].0; sy[i + 1] = sy[i] + p[i].1; }
    (0..n).map(|i| {
        let h = w.min(i).min(n - 1 - i);
        let (a, b) = (i - h, i + h + 1);
        ((sx[b] - sx[a]) / (b - a) as f32, (sy[b] - sy[a]) / (b - a) as f32)
    }).collect()
}

/// Split the samples into rests and moves. A rest is a run of samples within `r` px of its first
/// sample that lasts `REST_MS` (measured to the first sample that LEFT it, so a gap with no events
/// is a rest too) - or any run holding a click, however short, so a click mid-flight is still an
/// exact anchor. Moves fill the space between rests.
fn segment(pts: &[Pt], clicks: &[bool], r: f32) -> Vec<Seg> {
    let n = pts.len();
    let mut rests: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && dist(pts[j + 1], pts[i]) <= r { j += 1; }
        let end_t = if j + 1 < n { pts[j + 1].t } else { pts[j].t };
        if end_t - pts[i].t >= REST_MS || clicks[i..=j].iter().any(|&c| c) { rests.push((i, j)); i = j + 1; } else { i += 1; }
    }
    let mut segs: Vec<Seg> = Vec::new();
    let mut prev: Option<(usize, usize)> = None; // the last rest placed
    let push_move = |segs: &mut Vec<Seg>, i0: usize, i1: usize, t0: u32| {
        if i1 > i0 { segs.push(Seg { t0, t1: pts[i1].t, i0, i1, rest: false }); }
    };
    for &(a, b) in &rests {
        match prev {
            None => push_move(&mut segs, 0, a, pts[0].t),
            Some((_, pb)) => push_move(&mut segs, pb, a, move_start(pts, pb)),
        }
        // A rest holds until the next move starts; that start is patched in below once known.
        segs.push(Seg { t0: pts[a].t, t1: u32::MAX, i0: a, i1: b, rest: true });
        prev = Some((a, b));
    }
    match prev {
        None => if n > 0 { push_move(&mut segs, 0, n - 1, pts[0].t) },
        Some((_, pb)) => push_move(&mut segs, pb, n - 1, move_start(pts, pb)),
    }
    // Rests end where the following move begins (the segment list is what `at` searches by `t0`).
    for k in 0..segs.len() {
        if segs[k].rest { segs[k].t1 = segs.get(k + 1).map_or(u32::MAX, |m| m.t0); }
    }
    segs
}

/// When a move leaves the rest whose last sample is `pb`: just before the first sample that left.
fn move_start(pts: &[Pt], pb: usize) -> u32 {
    let next = pts.get(pb + 1).map_or(pts[pb].t, |p| p.t);
    next - (next - pts[pb].t).min(LEAD_MS)
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
