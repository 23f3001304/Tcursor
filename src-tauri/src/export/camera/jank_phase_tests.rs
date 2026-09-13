// Micro-probes for the hypotheses about PHASE boundaries inside one region: H1 (the aim function
// changes at ramp-in -> hold), H5 (ramp-out freezes the centre) and H6 (the in-frame clamp).
// Each drives `CameraSim` with a hand-written cursor so the printed number is that mechanism
// alone. Assertions pin the measured mechanism, never a taste judgement.
use super::jscene as js;
use super::cfg;
use crate::export::types::{FramePoint, ZoomRegion};

#[test]
fn h1_ramp_in_to_hold_on_a_follow_region() {
    // Cursor crosses the 350ms ramp-in -> hold boundary at a constant 0.6 px/ms (10 px/frame).
    let r = [js::micro_region(0, 4000, true)];
    let cur = |t: u32| FramePoint { x: 300 + (t as f32 * 0.6) as i32, y: 540 };
    let (ts, xs, _, _) = js::drive(&r, &cfg(), 150, 900, cur);
    let v = js::vel(&xs);
    println!("\n--- H1a  ramp-in -> hold, FOLLOW region (boundary t=350ms) ---");
    println!("  {:>6} {:>9} {:>9} {:>12}", "t", "cx", "v px/f", "cursor-cx");
    for (i, t) in ts.iter().enumerate() {
        if !(280..=620).contains(t) || i % 3 != 0 { continue; }
        println!("  {:>6} {:>9.1} {:>9.2} {:>12.1}", t, xs[i], v[i], (300.0 + *t as f32 * 0.6) - xs[i]);
    }
    let (before, after) = (js::band(&ts, &v, 250, 340), js::band(&ts, &v, 370, 700));
    println!("  mean |v| 250..340 = {before:.2} px/f, 370..700 = {after:.2} px/f");
    println!("  VERDICT (was): the live-cursor ramp landed the centre ON the cursor, i.e. INSIDE a\n\
              \x20 CAMERA-relative dead zone, so the follow froze at the boundary until the cursor\n\
              \x20 was 175px away (~290ms at this speed) - a velocity step from tracking to zero.\n\
              \x20 NOW: a follow region aims at the cursor in every phase (`follow::aim`), so the\n\
              \x20 boundary only swaps the eased ramp for the damped follow - no dead zone, no stop.");
    assert!(after > 5.0, "the follow must keep tracking past the boundary: {before} -> {after}");
}

#[test]
fn h1_ramp_in_to_hold_on_an_anchored_region() {
    // The same boundary for an ANCHORED region, with the cursor parked far from the anchor - the
    // aim used to switch from `r.anchor` to "dead zone around the cursor" in a single step.
    let r = [ZoomRegion { anchor: FramePoint { x: 1500, y: 540 }, ..js::micro_region(0, 4000, false) }];
    let cur = |_| FramePoint { x: 500, y: 540 };
    let (ts, xs, _, _) = js::drive(&r, &cfg(), 150, 1200, cur);
    let v = js::vel(&xs);
    println!("\n--- H1b  ramp-in -> hold, ANCHORED region (anchor 1500, cursor parked at 500) ---");
    println!("  {:>6} {:>9} {:>9} {:>12}", "t", "cx", "v px/f", "cursor-cx");
    for (i, t) in ts.iter().enumerate() {
        if !(280..=600).contains(t) || i % 3 != 0 { continue; }
        println!("  {:>6} {:>9.1} {:>9.2} {:>12.1}", t, xs[i], v[i], 500.0 - xs[i]);
    }
    let (before, after) = (js::band(&ts, &v, 250, 340), js::band(&ts, &v, 360, 450));
    println!("  mean |v| 250..340 = {before:.2} px/f, 360..450 = {after:.2} px/f");
    println!("  VERDICT (was): the hold phase ignored the anchor entirely - it only ever aimed at\n\
              \x20 the cursor's dead zone - so at the ramp's last step the target teleported from\n\
              \x20 1500 to 675 and the centre reversed at k*(tx-cx): the scene's largest spike.\n\
              \x20 NOW: the anchor IS the aim in every phase (`follow::aim`), so the ramp eases to\n\
              \x20 1500 and the hold's first target is where the ramp landed.");
    assert!(after < before, "the anchored hold must not yank toward the cursor: {before} -> {after}");
}

#[test]
fn h5_ramp_out_freezes_the_centre() {
    // Follow region, cursor still moving (0.25 px/ms) when the zoom-out ramp begins at end-450.
    let r = [js::micro_region(0, 3000, true)];
    let cur = |t: u32| FramePoint { x: 300 + (t as f32 * 0.25) as i32, y: 540 };
    let (ts, xs, _, ss) = js::drive(&r, &cfg(), 2200, 2900, cur);
    let v = js::vel(&xs);
    println!("\n--- H5  hold -> ramp-out (t=2550): the natural centre becomes the FROZEN self.cx ---");
    for (i, t) in ts.iter().enumerate() {
        if !(2400..=2750).contains(t) || i % 4 != 0 { continue; }
        println!("  {:>6} cx {:>9.1} v {:>8.3} scale {:.3}", t, xs[i], v[i], ss[i]);
    }
    let (before, after) = (js::band(&ts, &v, 2400, 2540), js::band(&ts, &v, 2560, 2700));
    println!("  mean |v| 2400..2540 = {before:.2} px/f, 2560..2700 = {after:.2} px/f");
    println!("  VERDICT (was): the cursor-follow was dropped the instant the ramp-out started, so\n\
              \x20 the camera stopped tracking mid-gesture (4.17 -> 0.00 px/f) and any remaining\n\
              \x20 centre motion was the in-frame clamp releasing as scale returned to 1 - which,\n\
              \x20 from a CLAMPED pose, is a large reverse sweep instead of a stop.\n\
              \x20 NOW: hold and ramp-out share the centre; only the SCALE eases back to 1, and the\n\
              \x20 clamp relaxes underneath a centre that is already heading the right way.");
    assert!(after > before * 0.5, "ramp-out must keep following: {before} -> {after}");
}

#[test]
fn h6_clamp_kinks_the_ramp() {
    // Anchored 2.6 zoom at (1500,1000): cy's clamp bound is 1080 - 1080/(2*2.6) = 872.3, so the
    // eased approach is truncated part-way through the ramp.
    // The cursor sits ON the anchor, so `follow::aim` returns the anchor untouched and the clamp
    // is the only thing that can truncate the ramp - which is what this probe is about.
    let r = [ZoomRegion { target_scale: 2.6, anchor: FramePoint { x: 1500, y: 1000 },
        ..js::micro_region(0, 3000, false) }];
    let (ts, _, ys, ss) = js::drive(&r, &cfg(), 0, 340, |_| FramePoint { x: 1500, y: 1000 });
    let v = js::vel(&ys);
    println!("\n--- H6  in-frame clamp during an anchored ramp-in (target cy 1000, bound 872.3) ---");
    let mut hit = 0u32;
    for (i, t) in ts.iter().enumerate() {
        let bound = js::FH as f32 - js::FH as f32 / (2.0 * ss[i]);
        if hit == 0 && (ys[i] - bound).abs() < 0.05 { hit = *t; }
        if *t % 50 < 17 { println!("  {:>6} cy {:>8.1} v {:>8.3} bound {:>8.1}", t, ys[i], v[i], bound); }
    }
    println!("  clamp engages at t={hit}ms; mean |v| before = {:.2}, after = {:.2} px/f",
        js::band(&ts, &v, hit.saturating_sub(100), hit.saturating_sub(1)), js::band(&ts, &v, hit, hit + 100));
    println!("  VERDICT: the ease curve is truncated mid-ramp; cy then tracks the MOVING bound\n\
              \x20 1080 - 540/scale instead of the ease, so its velocity is set by the SCALE ramp\n\
              \x20 from that instant on - a kink, not a stop, and it is invisible to `fit_durations`.");
    assert!(hit > 0, "this anchor must be clamped at scale 2.6");
}
