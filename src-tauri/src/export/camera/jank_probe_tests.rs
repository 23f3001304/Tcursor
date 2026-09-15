#[path = "jank_input_tests.rs"]
mod input;
#[path = "jank_metrics.rs"]
pub mod jm;
#[path = "jank_scene.rs"]
pub mod jscene;

use crate::export::types::ZoomConfig;

pub fn cfg() -> ZoomConfig {
    ZoomConfig::default()
}

#[test]
fn jank_probe_tables() {
    println!(
        "\n=== PART 1  jank probe: 12s synthetic scene, 1920x1080, cursor smoothness {} ===",
        jscene::SMOOTH_DEFAULT
    );
    println!(
        "  R1 follow_cursor 2.2 {:?}  R2 anchored 1.8 {:?}  R3 anchored 2.6 (corner) {:?}",
        jscene::R1,
        jscene::R2,
        jscene::R3
    );
    println!(
        "  units: screen px per frame (a pan of d_cx shows as scale*d_cx; a scale step ds\n\
              \x20 slides content at the viewport edge by FW/(2*scale)*ds)\n"
    );
    for g in [jscene::Grid::Export, jscene::Grid::Preview] {
        let r = jscene::run(g, &cfg(), jscene::SMOOTH_DEFAULT);
        println!("\n  --- {} : {} samples ---", g.name(), r.t.len());
        jm::print_jerk(g.name(), &r);
        let (a, b) = jm::window(&r, jscene::SWEEP.0, jscene::SWEEP.1);
        let s = jm::series(&r);
        println!(
            "  {:<22} {:>10.4} {:>10.4} {:>10.4}  (sweep {:?} only)",
            "  jerk rms px/f2",
            jm::rms(&s[0].acc[a..b]),
            jm::rms(&s[1].acc[a..b]),
            jm::rms(&s[2].acc[a..b]),
            jscene::SWEEP
        );
        jm::print_spikes(&r, 10);
    }
}

#[test]
fn preview_grid_vs_true_60fps() {
    let (e, p) = (
        jscene::run(jscene::Grid::Export, &cfg(), jscene::SMOOTH_DEFAULT),
        jscene::run(jscene::Grid::Preview, &cfg(), jscene::SMOOTH_DEFAULT),
    );
    let (mut dcx, mut dcy, mut ds, mut dpx, mut dcur) = (vec![], vec![], vec![], vec![], vec![]);
    let mut worst = (0u32, 0.0f32);
    for k in 0..=(jscene::DUR_MS / 4) {
        let t = k * 4;
        let (a, b) = (jm::at(&e, &e.cx, t), jm::at(&p, &p.cx, t));
        let s = jm::at(&e, &e.scale, t);
        if (a - b).abs() > worst.1 {
            worst = (t, (a - b).abs());
        }
        dcx.push((a - b).abs());
        dcy.push((jm::at(&e, &e.cy, t) - jm::at(&p, &p.cy, t)).abs());
        ds.push((s - jm::at(&p, &p.scale, t)).abs());
        dpx.push((a - b).abs() * s);
        dcur.push((jm::at(&e, &e.curx, t) - jm::at(&p, &p.curx, t)).abs());
    }
    let stat = |n: &str, v: &mut Vec<f32>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "  {n:<16} max {:>8.3}  p99 {:>8.3}  median {:>8.3}",
            v[v.len() - 1],
            v[v.len() * 99 / 100],
            v[v.len() / 2]
        );
    };
    println!("\n  --- preview fidelity (export 60fps vs camera_track, same instants) ---");
    stat("|d cx| src px", &mut dcx);
    stat("|d cy| src px", &mut dcy);
    stat("|d scale|", &mut ds);
    stat("|d cx| screen", &mut dpx);
    stat("|d cursor| px", &mut dcur);
    println!(
        "  worst |d cx| = {:.2} px at t={}ms (was 28.70 px at t=9344 on the old 16ms grid)",
        worst.1, worst.0
    );
    println!(
        "  preview clock at t=12000ms: export sample #{} vs preview sample #{}",
        e.t.len(),
        p.t.len()
    );
    assert_eq!(
        e.t.len(),
        p.t.len(),
        "the preview must sample the export's own frame grid"
    );
    assert!(
        worst.1 < 1e-3,
        "preview and export diverged by {:.3}px at t={}ms",
        worst.1,
        worst.0
    );
}

mod filter {
    use super::cfg;
    use super::jm;
    use super::jscene as js;
    use crate::export::types::ZoomConfig;

    fn fingerprint(r: &js::Run) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for i in 0..r.t.len() {
            for v in [r.scale[i], r.cx[i], r.cy[i]] {
                for b in v.to_bits().to_le_bytes() {
                    h = (h ^ b as u64).wrapping_mul(0x1000_0000_01b3);
                }
            }
        }
        h
    }

    #[test]
    fn smoothing_off_is_bit_identical() {
        let r = js::run(js::Grid::Export, &cfg(), js::SMOOTH_DEFAULT);
        assert_eq!(cfg().smoothing_ms, 0, "the default must stay off");
        assert_eq!(
            r.t.len(),
            721,
            "the probe's sample count changed - the pin is no longer comparable"
        );
        assert_eq!(
            fingerprint(&r),
            0x58da_34b7_41e9_0fd3,
            "smoothing_ms = 0 changed the shipped trajectory"
        );
    }

    fn best_lag(raw: &js::Run, f: &js::Run, t0: u32, t1: u32) -> (f32, f32) {
        let (i0, i1) = jm::window(raw, t0, t1);
        let mut best = (0usize, f32::MAX);
        for k in 0..40 {
            let (mut e, mut n) = (0.0f32, 0.0f32);
            for i in i0..i1 {
                if i + k < f.cx.len() {
                    e += (raw.cx[i] - f.cx[i + k]).powi(2);
                    n += 1.0;
                }
            }
            let r = (e / n.max(1.0)).sqrt();
            if r < best.1 {
                best = (k, r);
            }
        }
        (best.0 as f32 * 1000.0 / 60.0, best.1)
    }

    fn peak_vel_t(r: &js::Run, t0: u32, t1: u32) -> u32 {
        let (i0, i1) = jm::window(r, t0, t1);
        let v = js::vel(&r.cx);
        let mut best = (0usize, 0.0f32);
        for i in i0..i1 {
            if v[i].abs() > best.1 {
                best = (i, v[i].abs());
            }
        }
        r.t[best.0]
    }

    #[test]
    fn smoothing_before_after() {
        let raw = js::run(js::Grid::Export, &cfg(), js::SMOOTH_DEFAULT);
        println!("\n=== PART 2  critically-damped post-pass, export 60fps grid ===");
        println!("  whole run (12s), screen px per frame^2:");
        jm::print_jerk("smoothing_ms = 0", &raw);
        for ms in [120u32, 250] {
            let f = js::run(
                js::Grid::Export,
                &ZoomConfig {
                    smoothing_ms: ms,
                    ..cfg()
                },
                js::SMOOTH_DEFAULT,
            );
            jm::print_jerk(&format!("smoothing_ms = {ms}"), &f);
            let (lag, resid) = best_lag(&raw, &f, js::SWEEP.0, js::SWEEP.1 + 400);
            let (i0, i1) = jm::window(&raw, js::SWEEP.0, js::SWEEP.1 + 400);
            let err = (i0..i1)
                .map(|i| (raw.cx[i] - f.cx[i]).abs())
                .fold(0.0f32, f32::max);
            println!(
                "    lag: best-fit {lag:.0}ms (residual {resid:.1}px), velocity-peak delay {}ms, \
                max |d cx| vs raw {err:.0}px",
                peak_vel_t(&f, js::SWEEP.0, js::SWEEP.1 + 400) as i64
                    - peak_vel_t(&raw, js::SWEEP.0, js::SWEEP.1 + 400) as i64
            );
            let s = jm::series(&f);
            let (a, b) = jm::window(&f, js::SWEEP.0, js::SWEEP.1);
            println!(
                "    sweep-only jerk rms: cx {:.3} cy {:.3} scale {:.3}",
                jm::rms(&s[0].acc[a..b]),
                jm::rms(&s[1].acc[a..b]),
                jm::rms(&s[2].acc[a..b])
            );
            jm::print_spikes(&f, 5);
        }
    }

    #[test]
    fn smoothing_tames_the_worst_spike_but_cannot_remove_it() {
        let raw = js::run(js::Grid::Export, &cfg(), js::SMOOTH_DEFAULT);
        let worst = |r: &js::Run| {
            let s = jm::series(r);
            let (a, b) = jm::window(r, 9200, 9700);
            (
                jm::max_abs(&s[0].acc[a..b]).1,
                jm::max_abs(&s[1].acc[a..b]).1,
            )
        };
        let (bx, by) = worst(&raw);
        println!("\n--- worst spike (R3 ramp-in -> hold, 9200..9700ms), screen px/frame^2 ---");
        println!("  smoothing_ms =   0: cx {bx:.1}  cy {by:.1}");
        let mut prev = bx;
        for ms in [60u32, 120, 250, 400] {
            let f = js::run(
                js::Grid::Export,
                &ZoomConfig {
                    smoothing_ms: ms,
                    ..cfg()
                },
                js::SMOOTH_DEFAULT,
            );
            let (fx, fy) = worst(&f);
            println!(
                "  smoothing_ms = {ms:>3}: cx {fx:.1}  cy {fy:.1}   ({:+.0}% cx vs off)",
                (fx / bx - 1.0) * 100.0
            );
            assert!(
                fx < prev,
                "more smoothing must not raise the worst spike: {prev} -> {fx}"
            );
            prev = fx;
        }
        println!(
            "  NOTE: a causal filter can only spread a step over its settle time; removing one\n\
                  \x20 means fixing its cause. That is what happened here - making the hold phase's\n\
                  \x20 first target the pose the ramp just reached took this window from 129.8 to 6.7\n\
                  \x20 with NO filter at all, which no amount of smoothing had managed."
        );
    }
}
