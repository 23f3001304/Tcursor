// Jank metrics for the camera probe: first/second differences of a `Run`, expressed in the unit
// the viewer actually perceives (OUTPUT pixels of on-screen motion per frame), plus the spike
// ranking and the hypothesis classifier the report tables are built from.
use super::jscene::{Run, FW, R1, R2, R3, ZI, ZO};

/// One motion channel of a `Run`, converted to screen pixels. A pan of `dcx` source px shows as
/// `scale * dcx` px of motion; a scale step `ds` slides content at the viewport edge by
/// `half_extent * ds` = `FW/(2*scale) * ds`. `vel` is per frame, `acc` its first difference
/// (the jerk proxy) - both index-aligned with the run (leading entries 0).
pub struct Series { pub name: &'static str, pub vel: Vec<f32>, pub acc: Vec<f32> }

fn diff(v: &[f32]) -> Vec<f32> {
    let mut d = vec![0.0; v.len()];
    for i in 1..v.len() { d[i] = v[i] - v[i - 1]; }
    d
}

pub fn series(r: &Run) -> Vec<Series> {
    let px = |v: &[f32], k: &dyn Fn(usize) -> f32| {
        let d = diff(v);
        (0..d.len()).map(|i| d[i] * k(i)).collect::<Vec<f32>>()
    };
    let s = r.scale.clone();
    let vels = vec![
        ("cx", px(&r.cx, &|i| s[i])),
        ("cy", px(&r.cy, &|i| s[i])),
        ("scale", px(&r.scale, &|i| FW as f32 / (2.0 * s[i].max(0.01)))),
    ];
    vels.into_iter().map(|(name, vel)| { let acc = diff(&vel); Series { name, vel, acc } }).collect()
}

pub fn rms(v: &[f32]) -> f32 {
    if v.is_empty() { return 0.0; }
    (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt()
}

pub fn max_abs(v: &[f32]) -> (usize, f32) {
    v.iter().enumerate().fold((0, 0.0), |(bi, bv), (i, x)| if x.abs() > bv { (i, x.abs()) } else { (bi, bv) })
}

/// Index range `[a, b)` covering the ms window `[t0, t1]`.
pub fn window(r: &Run, t0: u32, t1: u32) -> (usize, usize) {
    let a = r.t.partition_point(|&t| t < t0);
    let b = r.t.partition_point(|&t| t <= t1);
    (a, b.max(a))
}

/// Linear interpolation of `v` at output time `t` - exactly what the editor's `camAt` does
/// between `camera_track` samples, so a preview run can be compared to an export run at the
/// SAME instant instead of at whatever timestamps each grid happens to land on.
pub fn at(r: &Run, v: &[f32], t: u32) -> f32 {
    let i = r.t.partition_point(|&s| s <= t);
    if i == 0 { return v[0]; }
    if i >= r.t.len() { return v[v.len() - 1]; }
    let (t0, t1) = (r.t[i - 1] as f32, r.t[i] as f32);
    let f = ((t as f32 - t0) / (t1 - t0).max(1.0)).clamp(0.0, 1.0);
    v[i - 1] + (v[i] - v[i - 1]) * f
}

pub struct Spike { pub t: u32, pub i: usize, pub comp: &'static str, pub acc: f32, pub v0: f32, pub v1: f32 }

/// The `n` largest |acc| events across all three channels, suppressing the ringing right after a
/// spike (a same-channel sample within 2 steps of one already taken) so the table shows `n`
/// distinct events rather than one event's decay.
pub fn top_spikes(r: &Run, n: usize) -> Vec<Spike> {
    let ss = series(r);
    let mut all: Vec<Spike> = Vec::new();
    for s in &ss {
        for i in 2..s.acc.len() {
            all.push(Spike { t: r.t[i], i, comp: s.name, acc: s.acc[i], v0: s.vel[i - 1], v1: s.vel[i] });
        }
    }
    all.sort_by(|a, b| b.acc.abs().partial_cmp(&a.acc.abs()).unwrap());
    let mut out: Vec<Spike> = Vec::new();
    for sp in all {
        if out.iter().any(|o| o.comp == sp.comp && o.i.abs_diff(sp.i) <= 2) { continue; }
        out.push(sp);
        if out.len() == n { break; }
    }
    out
}

/// Which hypothesis a spike at sample `i` belongs to, decided from the run's own recorded state
/// (driver index, clamp flag, cursor vs camera distance) and the region timetable - not by eye.
pub fn classify(r: &Run, i: usize) -> String {
    let t = r.t[i];
    let near = |x: u32| t.abs_diff(x) <= 34; // within two frames of a known boundary
    if i > 0 && r.driver[i] != r.driver[i - 1] {
        return format!("H2 handoff driver {} -> {}", r.driver[i - 1], r.driver[i]);
    }
    for (idx, (s, e)) in [R1, R2, R3].iter().enumerate() {
        if r.driver[i] != idx as i32 { continue; }
        if near(s + ZI) { return "H1 ramp-in -> hold".into(); }
        if near(e - ZO) { return "H5 hold -> ramp-out (centre freezes)".into(); }
        if near(*s) { return "H2 region start (fresh ramp from rest)".into(); }
        if near(*e) { return "H6/H5 region end".into(); }
    }
    if i > 0 && r.clamped[i] != r.clamped[i - 1] {
        return format!("H6 clamp {}", if r.clamped[i] { "engaged" } else { "released" });
    }
    if r.clamped[i] { return "H6 clamp riding the frame edge".into(); }
    "H4 cursor low-pass / raw sample jitter".into()
}

pub fn print_spikes(r: &Run, n: usize) {
    println!("\n  top {} |dv| spikes - {} grid", n, r.grid.name());
    println!("  {:>5} {:>8} {:>6} {:>10} {:>10} {:>10}  {}", "#", "t(ms)", "chan",
        "v[i-1]px", "v[i]px", "dv px/f2", "classification");
    for (k, sp) in top_spikes(r, n).iter().enumerate() {
        println!("  {:>5} {:>8} {:>6} {:>10.3} {:>10.3} {:>10.3}  {}", k + 1, sp.t, sp.comp,
            sp.v0, sp.v1, sp.acc, classify(r, sp.i));
    }
}

/// Whole-run jerk summary per channel, in screen px/frame^2.
pub fn print_jerk(tag: &str, r: &Run) {
    println!("  {:<22} {:>10} {:>10} {:>10}", tag, "rms(cx)", "rms(cy)", "rms(scale)");
    let ss = series(r);
    let v: Vec<f32> = ss.iter().map(|s| rms(&s.acc)).collect();
    println!("  {:<22} {:>10.4} {:>10.4} {:>10.4}", "  jerk rms px/f2", v[0], v[1], v[2]);
    let m: Vec<f32> = ss.iter().map(|s| max_abs(&s.acc).1).collect();
    println!("  {:<22} {:>10.4} {:>10.4} {:>10.4}", "  jerk max px/f2", m[0], m[1], m[2]);
}
