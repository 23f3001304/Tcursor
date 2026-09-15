use super::cfg;
use super::jm;
use super::jscene as js;
use crate::export::camera::follow::damping;
use crate::export::cursor::Cursor;
use crate::export::types::{FramePoint, ZoomRegion};

#[test]
fn h2_handoff_restarts_from_zero_velocity() {
    let cur = |t: u32| FramePoint {
        x: 300 + (t as f32 * 0.6) as i32,
        y: 540,
    };
    let a = js::micro_region(0, 6000, true);
    let b = ZoomRegion {
        start_ms: 2000,
        end_ms: 5000,
        layer: 1,
        anchor: FramePoint { x: 1500, y: 540 },
        follow_cursor: false,
        ..a
    };
    let (ts, xs, _, _) = js::drive(&[a, b], &cfg(), 1700, 2600, cur);
    let v = js::vel(&xs);
    println!("\n--- H2  handoff Transition starts from velocity 0 (winner changes at t=2000) ---");
    println!("  {:>6} {:>9} {:>9}", "t", "cx", "v px/f");
    for (i, t) in ts.iter().enumerate() {
        if !(1900..=2250).contains(t) || i % 3 != 0 {
            continue;
        }
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
    assert!(
        after > before * 0.8,
        "handoff must not stall the pan: {before} -> {after}"
    );
}

#[test]
fn h4_cursor_low_pass_is_per_step_not_per_ms() {
    let tau = |a: f32, dt: f32| -dt / (1.0f32 - a).ln();
    println!("\n--- H4  the camera follow alpha, per STEP (was) vs per millisecond (now) ---");
    println!(
        "  {:>7} {:>13} {:>13} {:>9}  {:>13} {:>13} {:>9}",
        "alpha", "raw tau@16.667", "raw tau@16", "gap", "tau@16.667", "tau@30fps", "gap"
    );
    for a in [0.10f32, 0.36, 0.75] {
        let (t0, t1) = (tau(a, 16.667), tau(a, 16.0));
        let fixed = |dt: f32| tau(damping(a, dt), dt);
        println!(
            "  {a:>7.2} {t0:>11.1}ms {t1:>11.1}ms {:>8.1}%  {:>11.1}ms {:>11.1}ms {:>8.1}%",
            (t1 / t0 - 1.0) * 100.0,
            fixed(16.667),
            fixed(1000.0 / 30.0),
            (fixed(1000.0 / 30.0) / fixed(16.667) - 1.0) * 100.0
        );
    }
    println!(
        "  smoothness 0.00: no glide at all (plain-OS cursor mode) - raw samples reach the camera"
    );
    for a in [1.0f32, js::SMOOTH_DEFAULT, 0.0] {
        let mut c = Cursor::new(js::events(), js::screen(), a);
        let mut xs = vec![];
        for i in 0.. {
            let t = (i as u64 * 1000 / 60) as u32;
            if t > 5900 {
                break;
            }
            let p = c.at(t, js::STEP_MS);
            if t >= 3700 {
                xs.push(p.x as f32);
            }
        }
        println!(
            "  smoothness {a:.2}: cursor accel rms over the near-still pause = {:.3} px/f2",
            jm::rms(&js::vel(&js::vel(&xs)))
        );
    }
    println!("  VERDICT: a fixed per-step alpha smoothed ~4.0% harder on a 16ms grid than on the\n\
              \x20 export's 16.667ms one (same alpha, more steps per second), and the gap grew to\n\
              \x20 ~100% at a 30fps export. `Cursor::at` now takes the caller's EXACT frame period\n\
              \x20 and converts through `follow::damping` (the camera's own conversion), so the\n\
              \x20 time constant is identical at every rate - the right-hand gap column is 0.0%.\n\
              \x20 At alpha 1.0 raw +-2px sample jitter still reaches the follow directly: that is\n\
              \x20 plain-OS mode asking for no low-pass, not a defect.");
    for a in [0.10f32, 0.36, 0.75] {
        for dt in [8.0f32, 16.0, 16.667, 1000.0 / 30.0, 50.0] {
            assert!(
                (tau(damping(a, dt), dt) / tau(a, 16.66667) - 1.0).abs() < 2e-3,
                "alpha {a} at dt {dt}ms no longer means the same time constant"
            );
        }
    }
}

mod phase {
    use super::cfg;
    use super::js;
    use crate::export::types::{FramePoint, ZoomRegion};

    #[test]
    fn h1_ramp_in_to_hold_on_a_follow_region() {
        let r = [js::micro_region(0, 4000, true)];
        let cur = |t: u32| FramePoint {
            x: 300 + (t as f32 * 0.6) as i32,
            y: 540,
        };
        let (ts, xs, _, _) = js::drive(&r, &cfg(), 150, 900, cur);
        let v = js::vel(&xs);
        println!("\n--- H1a  ramp-in -> hold, FOLLOW region (boundary t=350ms) ---");
        println!(
            "  {:>6} {:>9} {:>9} {:>12}",
            "t", "cx", "v px/f", "cursor-cx"
        );
        for (i, t) in ts.iter().enumerate() {
            if !(280..=620).contains(t) || i % 3 != 0 {
                continue;
            }
            println!(
                "  {:>6} {:>9.1} {:>9.2} {:>12.1}",
                t,
                xs[i],
                v[i],
                (300.0 + *t as f32 * 0.6) - xs[i]
            );
        }
        let (before, after) = (js::band(&ts, &v, 250, 340), js::band(&ts, &v, 370, 700));
        println!("  mean |v| 250..340 = {before:.2} px/f, 370..700 = {after:.2} px/f");
        println!("  VERDICT (was): the live-cursor ramp landed the centre ON the cursor, i.e. INSIDE a\n\
                  \x20 CAMERA-relative dead zone, so the follow froze at the boundary until the cursor\n\
                  \x20 was 175px away (~290ms at this speed) - a velocity step from tracking to zero.\n\
                  \x20 NOW: a follow region aims at the cursor in every phase (`follow::aim`), so the\n\
                  \x20 boundary only swaps the eased ramp for the damped follow - no dead zone, no stop.");
        assert!(
            after > 5.0,
            "the follow must keep tracking past the boundary: {before} -> {after}"
        );
    }

    #[test]
    fn h1_ramp_in_to_hold_on_an_anchored_region() {
        let r = [ZoomRegion {
            anchor: FramePoint { x: 1500, y: 540 },
            ..js::micro_region(0, 4000, false)
        }];
        let cur = |_| FramePoint { x: 500, y: 540 };
        let (ts, xs, _, _) = js::drive(&r, &cfg(), 150, 1200, cur);
        let v = js::vel(&xs);
        println!(
            "\n--- H1b  ramp-in -> hold, ANCHORED region (anchor 1500, cursor parked at 500) ---"
        );
        println!(
            "  {:>6} {:>9} {:>9} {:>12}",
            "t", "cx", "v px/f", "cursor-cx"
        );
        for (i, t) in ts.iter().enumerate() {
            if !(280..=600).contains(t) || i % 3 != 0 {
                continue;
            }
            println!(
                "  {:>6} {:>9.1} {:>9.2} {:>12.1}",
                t,
                xs[i],
                v[i],
                500.0 - xs[i]
            );
        }
        let (before, after) = (js::band(&ts, &v, 250, 340), js::band(&ts, &v, 360, 450));
        println!("  mean |v| 250..340 = {before:.2} px/f, 360..450 = {after:.2} px/f");
        println!("  VERDICT (was): the hold phase ignored the anchor entirely - it only ever aimed at\n\
                  \x20 the cursor's dead zone - so at the ramp's last step the target teleported from\n\
                  \x20 1500 to 675 and the centre reversed at k*(tx-cx): the scene's largest spike.\n\
                  \x20 NOW: the anchor IS the aim in every phase (`follow::aim`), so the ramp eases to\n\
                  \x20 1500 and the hold's first target is where the ramp landed.");
        assert!(
            after < before,
            "the anchored hold must not yank toward the cursor: {before} -> {after}"
        );
    }

    #[test]
    fn h5_ramp_out_freezes_the_centre() {
        let r = [js::micro_region(0, 3000, true)];
        let cur = |t: u32| FramePoint {
            x: 300 + (t as f32 * 0.25) as i32,
            y: 540,
        };
        let (ts, xs, _, ss) = js::drive(&r, &cfg(), 2200, 2900, cur);
        let v = js::vel(&xs);
        println!(
            "\n--- H5  hold -> ramp-out (t=2550): the natural centre becomes the FROZEN self.cx ---"
        );
        for (i, t) in ts.iter().enumerate() {
            if !(2400..=2750).contains(t) || i % 4 != 0 {
                continue;
            }
            println!(
                "  {:>6} cx {:>9.1} v {:>8.3} scale {:.3}",
                t, xs[i], v[i], ss[i]
            );
        }
        let (before, after) = (js::band(&ts, &v, 2400, 2540), js::band(&ts, &v, 2560, 2700));
        println!("  mean |v| 2400..2540 = {before:.2} px/f, 2560..2700 = {after:.2} px/f");
        println!("  VERDICT (was): the cursor-follow was dropped the instant the ramp-out started, so\n\
                  \x20 the camera stopped tracking mid-gesture (4.17 -> 0.00 px/f) and any remaining\n\
                  \x20 centre motion was the in-frame clamp releasing as scale returned to 1 - which,\n\
                  \x20 from a CLAMPED pose, is a large reverse sweep instead of a stop.\n\
                  \x20 NOW: hold and ramp-out share the centre; only the SCALE eases back to 1, and the\n\
                  \x20 clamp relaxes underneath a centre that is already heading the right way.");
        assert!(
            after > before * 0.5,
            "ramp-out must keep following: {before} -> {after}"
        );
    }

    #[test]
    fn h6_clamp_kinks_the_ramp() {
        let r = [ZoomRegion {
            target_scale: 2.6,
            anchor: FramePoint { x: 1500, y: 1000 },
            ..js::micro_region(0, 3000, false)
        }];
        let (ts, _, ys, ss) = js::drive(&r, &cfg(), 0, 340, |_| FramePoint { x: 1500, y: 1000 });
        let v = js::vel(&ys);
        println!(
            "\n--- H6  in-frame clamp during an anchored ramp-in (target cy 1000, bound 872.3) ---"
        );
        let mut hit = 0u32;
        for (i, t) in ts.iter().enumerate() {
            let bound = js::FH as f32 - js::FH as f32 / (2.0 * ss[i]);
            if hit == 0 && (ys[i] - bound).abs() < 0.05 {
                hit = *t;
            }
            if *t % 50 < 17 {
                println!(
                    "  {:>6} cy {:>8.1} v {:>8.3} bound {:>8.1}",
                    t, ys[i], v[i], bound
                );
            }
        }
        println!(
            "  clamp engages at t={hit}ms; mean |v| before = {:.2}, after = {:.2} px/f",
            js::band(&ts, &v, hit.saturating_sub(100), hit.saturating_sub(1)),
            js::band(&ts, &v, hit, hit + 100)
        );
        println!("  VERDICT: the ease curve is truncated mid-ramp; cy then tracks the MOVING bound\n\
                  \x20 1080 - 540/scale instead of the ease, so its velocity is set by the SCALE ramp\n\
                  \x20 from that instant on - a kink, not a stop, and it is invisible to `fit_durations`.");
        assert!(hit > 0, "this anchor must be clamped at scale 2.6");
    }
}
