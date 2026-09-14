// PART 2: the opt-in critically-damped post-pass (`ZoomConfig::smoothing_ms`). Pins that 0 is
// bit-identical to the un-filtered camera, then prints the before/after jerk and the lag the
// filter costs - the trade-off a product knob would have to expose.
use super::jm;
use super::jscene as js;
use super::cfg;
use crate::export::types::ZoomConfig;

/// FNV-1a over every `(scale, cx, cy)` bit pattern of a whole export-grid run: one number that
/// changes if ANY sample of the shipped trajectory moves, even by one ulp.
fn fingerprint(r: &js::Run) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for i in 0..r.t.len() {
        for v in [r.scale[i], r.cx[i], r.cy[i]] {
            for b in v.to_bits().to_le_bytes() { h = (h ^ b as u64).wrapping_mul(0x1000_0000_01b3); }
        }
    }
    h
}

#[test]
fn smoothing_off_is_bit_identical() {
    // `smoothing_ms: 0` must not move a single sample of the shipped camera - not "within 1e-3",
    // bit for bit. The value is re-captured DELIBERATELY whenever the trajectory itself is meant
    // to change; it was `0x4ff9_a792_835b_3c1f` before the H1 aim-continuity fix (`follow::aim`
    // made the zoom-in ramp land on the pose the hold phase holds) and `0xc697_bcbc_2f38_488c`
    // before the H2 velocity carry (`handoff.rs`) and `0xf46a_3964_180c_e19e` before the H5 fix
    // (the zoom-out ramp keeps following instead of freezing the centre) and
    // `0x009e_462b_a5df_652f` before H4 made `follow_damping` a per-millisecond time constant, and
    // `0x36bd_cdf0_69b1_b341` before the sim started taking the EXACT frame period from its caller
    // instead of differencing whole-millisecond timestamps, and `0xf32c_73d4_f690_57e7` before an
    // anchored region stopped sliding toward a far-away cursor (`follow::aim` is anchor-only), and
    // `0xf680_77c4_4f43_36fb` before the scale was floored at full frame (the `Some -> None`
    // handoff's velocity carry dipped it a hair under 1.0 after every ramp-out), and
    // `0x0824_ea72_2c9b_d85f` before the cursor stopped being a lagging low-pass and became the
    // rest/move path model (`export/cursor/path.rs`) - the camera follows a cursor that now
    // arrives with the recording, so every sample of the trajectory moved on purpose.
    let r = js::run(js::Grid::Export, &cfg(), js::SMOOTH_DEFAULT);
    assert_eq!(cfg().smoothing_ms, 0, "the default must stay off");
    assert_eq!(r.t.len(), 721, "the probe's sample count changed - the pin is no longer comparable");
    assert_eq!(fingerprint(&r), 0x58da_34b7_41e9_0fd3, "smoothing_ms = 0 changed the shipped trajectory");
}

/// Frames the filtered run must be advanced by to best match the raw one over `[t0, t1]`, i.e.
/// the lag the filter introduces, plus the residual rms once that lag is taken out.
fn best_lag(raw: &js::Run, f: &js::Run, t0: u32, t1: u32) -> (f32, f32) {
    let (i0, i1) = jm::window(raw, t0, t1);
    let mut best = (0usize, f32::MAX);
    for k in 0..40 {
        let (mut e, mut n) = (0.0f32, 0.0f32);
        for i in i0..i1 {
            if i + k < f.cx.len() { e += (raw.cx[i] - f.cx[i + k]).powi(2); n += 1.0; }
        }
        let r = (e / n.max(1.0)).sqrt();
        if r < best.1 { best = (k, r); }
    }
    (best.0 as f32 * 1000.0 / 60.0, best.1)
}

/// Time of the largest |d cx| inside `[t0, t1]` - the sweep's velocity peak.
fn peak_vel_t(r: &js::Run, t0: u32, t1: u32) -> u32 {
    let (i0, i1) = jm::window(r, t0, t1);
    let v = js::vel(&r.cx);
    let mut best = (0usize, 0.0f32);
    for i in i0..i1 { if v[i].abs() > best.1 { best = (i, v[i].abs()); } }
    r.t[best.0]
}

#[test]
fn smoothing_before_after() {
    let raw = js::run(js::Grid::Export, &cfg(), js::SMOOTH_DEFAULT);
    println!("\n=== PART 2  critically-damped post-pass, export 60fps grid ===");
    println!("  whole run (12s), screen px per frame^2:");
    jm::print_jerk("smoothing_ms = 0", &raw);
    for ms in [120u32, 250] {
        let f = js::run(js::Grid::Export, &ZoomConfig { smoothing_ms: ms, ..cfg() }, js::SMOOTH_DEFAULT);
        jm::print_jerk(&format!("smoothing_ms = {ms}"), &f);
        let (lag, resid) = best_lag(&raw, &f, js::SWEEP.0, js::SWEEP.1 + 400);
        let (i0, i1) = jm::window(&raw, js::SWEEP.0, js::SWEEP.1 + 400);
        let err = (i0..i1).map(|i| (raw.cx[i] - f.cx[i]).abs()).fold(0.0f32, f32::max);
        println!("    lag: best-fit {lag:.0}ms (residual {resid:.1}px), velocity-peak delay {}ms, \
            max |d cx| vs raw {err:.0}px",
            peak_vel_t(&f, js::SWEEP.0, js::SWEEP.1 + 400) as i64
                - peak_vel_t(&raw, js::SWEEP.0, js::SWEEP.1 + 400) as i64);
        let s = jm::series(&f);
        let (a, b) = jm::window(&f, js::SWEEP.0, js::SWEEP.1);
        println!("    sweep-only jerk rms: cx {:.3} cy {:.3} scale {:.3}",
            jm::rms(&s[0].acc[a..b]), jm::rms(&s[1].acc[a..b]), jm::rms(&s[2].acc[a..b]));
        jm::print_spikes(&f, 5);
    }
}

#[test]
fn smoothing_tames_the_worst_spike_but_cannot_remove_it() {
    // t=9350 is R3's ramp-in -> hold boundary. It USED to be the scene's largest |dv| by a wide
    // margin (H1b, cx 129.8 px/f2); `follow::aim` removed the aim discontinuity itself and the
    // window is down to 6.7. What is left is the residue a causal post-pass can only spread over
    // its settle time - which is exactly the point: more smoothing must never raise it.
    let raw = js::run(js::Grid::Export, &cfg(), js::SMOOTH_DEFAULT);
    let worst = |r: &js::Run| {
        let s = jm::series(r);
        let (a, b) = jm::window(r, 9200, 9700);
        (jm::max_abs(&s[0].acc[a..b]).1, jm::max_abs(&s[1].acc[a..b]).1)
    };
    let (bx, by) = worst(&raw);
    println!("\n--- worst spike (R3 ramp-in -> hold, 9200..9700ms), screen px/frame^2 ---");
    println!("  smoothing_ms =   0: cx {bx:.1}  cy {by:.1}");
    let mut prev = bx;
    for ms in [60u32, 120, 250, 400] {
        let f = js::run(js::Grid::Export, &ZoomConfig { smoothing_ms: ms, ..cfg() }, js::SMOOTH_DEFAULT);
        let (fx, fy) = worst(&f);
        println!("  smoothing_ms = {ms:>3}: cx {fx:.1}  cy {fy:.1}   ({:+.0}% cx vs off)",
            (fx / bx - 1.0) * 100.0);
        assert!(fx < prev, "more smoothing must not raise the worst spike: {prev} -> {fx}");
        prev = fx;
    }
    println!("  NOTE: a causal filter can only spread a step over its settle time; removing one\n\
              \x20 means fixing its cause. That is what happened here - making the hold phase's\n\
              \x20 first target the pose the ramp just reached took this window from 129.8 to 6.7\n\
              \x20 with NO filter at all, which no amount of smoothing had managed.");
}
