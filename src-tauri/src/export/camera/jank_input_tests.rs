// Micro-probes for the hypotheses about what is FED to the camera rather than what happens
// inside one region: H2 (a handoff `Transition` restarts from velocity 0) and H4 (the cursor
// low-pass alpha being per-step, plus raw sample jitter at low smoothness). The H3 probe (the
// dead-zone edge as a step input to the follow lerp) went with the dead zone itself: an
// anchored region no longer reacts to the cursor at all (`follow::aim`).
use super::jm;
use super::jscene as js;
use super::cfg;
use crate::export::camera::follow::damping;
use crate::export::cursor::Cursor;
use crate::export::types::{FramePoint, ZoomRegion};

#[test]
fn h2_handoff_restarts_from_zero_velocity() {
    // A is a follow region panning steadily; higher-layer B takes over mid-pan at t=2000.
    let cur = |t: u32| FramePoint { x: 300 + (t as f32 * 0.6) as i32, y: 540 };
    let a = js::micro_region(0, 6000, true);
    let b = ZoomRegion { start_ms: 2000, end_ms: 5000, layer: 1, anchor: FramePoint { x: 1500, y: 540 },
        follow_cursor: false, ..a };
    let (ts, xs, _, _) = js::drive(&[a, b], &cfg(), 1700, 2600, cur);
    let v = js::vel(&xs);
    println!("\n--- H2  handoff Transition starts from velocity 0 (winner changes at t=2000) ---");
    println!("  {:>6} {:>9} {:>9}", "t", "cx", "v px/f");
    for (i, t) in ts.iter().enumerate() {
        if !(1900..=2250).contains(t) || i % 3 != 0 { continue; }
        println!("  {:>6} {:>9.1} {:>9.2}", t, xs[i], v[i]);
    }
    let (before, after) = (js::band(&ts, &v, 1850, 1990), js::band(&ts, &v, 2000, 2080));
    println!("  mean |v| 1850..1990 = {before:.2} px/f, 2000..2080 = {after:.2} px/f");
    println!("  VERDICT (was): `Transition` interpolated POSITION from the pose captured at the\n\
              \x20 handoff, and ease(0)=0 with ease'(0)=0 for smoothstep, so the blend contributed\n\
              \x20 no velocity on its first steps: a moving camera was stopped dead, then\n\
              \x20 re-launched (10.00 -> 5.13 px/f, with a literal 0.00 at t=2000).\n\
              \x20 NOW: the sim records its own per-ms velocity and the blend adds a Hermite term\n\
              \x20 `u(u-1)^2 * dur * v0` (`handoff.rs`) - zero at both ends, slope v0 at the start.");
    assert!(after > before * 0.8, "handoff must not stall the pan: {before} -> {after}");
}

#[test]
fn h4_cursor_low_pass_is_per_step_not_per_ms() {
    let tau = |a: f32, dt: f32| -dt / (1.0f32 - a).ln();
    println!("\n--- H4  the cursor low-pass alpha, per STEP (was) vs per millisecond (now) ---");
    println!("  {:>7} {:>13} {:>13} {:>9}  {:>13} {:>13} {:>9}", "alpha",
        "raw tau@16.667", "raw tau@16", "gap", "tau@16.667", "tau@30fps", "gap");
    for a in [0.10f32, js::ALPHA_DEFAULT, 0.75] {
        let (t0, t1) = (tau(a, 16.667), tau(a, 16.0));
        // What `Cursor::at` does now: convert the per-60fps-frame alpha to THIS step first.
        let fixed = |dt: f32| tau(damping(a, dt), dt);
        println!("  {a:>7.2} {t0:>11.1}ms {t1:>11.1}ms {:>8.1}%  {:>11.1}ms {:>11.1}ms {:>8.1}%",
            (t1 / t0 - 1.0) * 100.0, fixed(16.667), fixed(1000.0 / 30.0),
            (fixed(1000.0 / 30.0) / fixed(16.667) - 1.0) * 100.0);
    }
    println!("  alpha 1.00: no low-pass at all (plain-OS cursor mode) - raw samples reach the camera");
    for a in [0.10f32, js::ALPHA_DEFAULT, 1.0] {
        let mut c = Cursor::new(js::events(), js::screen(), a);
        let mut xs = vec![];
        for i in 0.. {
            let t = (i as u64 * 1000 / 60) as u32;
            if t > 5900 { break; }
            let p = c.at(t, js::STEP_MS);
            if t >= 3700 { xs.push(p.x as f32); }
        }
        println!("  alpha {a:.2}: cursor accel rms over the near-still pause = {:.3} px/f2",
            jm::rms(&js::vel(&js::vel(&xs))));
    }
    println!("  VERDICT: a fixed per-step alpha smoothed ~4.0% harder on a 16ms grid than on the\n\
              \x20 export's 16.667ms one (same alpha, more steps per second), and the gap grew to\n\
              \x20 ~100% at a 30fps export. `Cursor::at` now takes the caller's EXACT frame period\n\
              \x20 and converts through `follow::damping` (the camera's own conversion), so the\n\
              \x20 time constant is identical at every rate - the right-hand gap column is 0.0%.\n\
              \x20 At alpha 1.0 raw +-2px sample jitter still reaches the follow directly: that is\n\
              \x20 plain-OS mode asking for no low-pass, not a defect.");
    for a in [0.10f32, js::ALPHA_DEFAULT, 0.75] {
        for dt in [8.0f32, 16.0, 16.667, 1000.0 / 30.0, 50.0] {
            assert!((tau(damping(a, dt), dt) / tau(a, 16.66667) - 1.0).abs() < 2e-3,
                "alpha {a} at dt {dt}ms no longer means the same time constant");
        }
    }
}
