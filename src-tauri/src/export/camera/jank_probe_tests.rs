// PART 1 of the camera-jank investigation: drive the synthetic scene (`jank_scene.rs`) through
// `CameraSim` on BOTH sample grids and print the numbers the report is built from - whole-run
// jerk, the top |dv| spikes with a hypothesis attached to each, and how far the editor preview's
// 16ms grid drifts from the exporter's own 60fps one. Run with:
//   cargo test -p tcursor-scaffold --lib jank -- --nocapture --test-threads=1
#[path = "jank_scene.rs"] pub mod jscene;
#[path = "jank_metrics.rs"] pub mod jm;
#[path = "jank_phase_tests.rs"] mod phase;
#[path = "jank_input_tests.rs"] mod input;
#[path = "jank_filter_tests.rs"] mod filter;

use crate::export::types::ZoomConfig;

pub fn cfg() -> ZoomConfig { ZoomConfig::default() } // follow_damping 0.10, the shipped default

#[test]
fn jank_probe_tables() {
    println!("\n=== PART 1  jank probe: 12s synthetic scene, 1920x1080, cursor alpha {} ===",
        jscene::ALPHA_DEFAULT);
    println!("  R1 follow_cursor 2.2 {:?}  R2 anchored 1.8 {:?}  R3 anchored 2.6 (corner) {:?}",
        jscene::R1, jscene::R2, jscene::R3);
    println!("  units: screen px per frame (a pan of d_cx shows as scale*d_cx; a scale step ds\n\
              \x20 slides content at the viewport edge by FW/(2*scale)*ds)\n");
    for g in [jscene::Grid::Export, jscene::Grid::Preview] {
        let r = jscene::run(g, &cfg(), jscene::ALPHA_DEFAULT);
        println!("\n  --- {} : {} samples ---", g.name(), r.t.len());
        jm::print_jerk(g.name(), &r);
        let (a, b) = jm::window(&r, jscene::SWEEP.0, jscene::SWEEP.1);
        let s = jm::series(&r);
        println!("  {:<22} {:>10.4} {:>10.4} {:>10.4}  (sweep {:?} only)", "  jerk rms px/f2",
            jm::rms(&s[0].acc[a..b]), jm::rms(&s[1].acc[a..b]), jm::rms(&s[2].acc[a..b]), jscene::SWEEP);
        jm::print_spikes(&r, 10);
    }
}

#[test]
fn preview_grid_vs_true_60fps() {
    // The editor preview USED to step `step_camera` on a flat 16ms grid (`1000 / OUT_FPS` in
    // integer math) while the exporter stepped `k * 1000 / 60`. Both are stateful and both lerped
    // per STEP, so the preview took 4.17% more steps per second than the export ever does and
    // drifted from it by up to 28.7 source px (74.5 screen px) at the sharp transitions.
    // `camera_track` now walks the export's own frame index at the exact 16.667ms period, so the
    // two are the same grid and this compares them the way the editor does - interpolated at the
    // same instants - to pin that they agree exactly.
    let (e, p) = (jscene::run(jscene::Grid::Export, &cfg(), jscene::ALPHA_DEFAULT),
        jscene::run(jscene::Grid::Preview, &cfg(), jscene::ALPHA_DEFAULT));
    let (mut dcx, mut dcy, mut ds, mut dpx, mut dcur) = (vec![], vec![], vec![], vec![], vec![]);
    let mut worst = (0u32, 0.0f32);
    for k in 0..=(jscene::DUR_MS / 4) {
        let t = k * 4;
        let (a, b) = (jm::at(&e, &e.cx, t), jm::at(&p, &p.cx, t));
        let s = jm::at(&e, &e.scale, t);
        if (a - b).abs() > worst.1 { worst = (t, (a - b).abs()); }
        dcx.push((a - b).abs());
        dcy.push((jm::at(&e, &e.cy, t) - jm::at(&p, &p.cy, t)).abs());
        ds.push((s - jm::at(&p, &p.scale, t)).abs());
        dpx.push((a - b).abs() * s);
        dcur.push((jm::at(&e, &e.curx, t) - jm::at(&p, &p.curx, t)).abs());
    }
    let stat = |n: &str, v: &mut Vec<f32>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("  {n:<16} max {:>8.3}  p99 {:>8.3}  median {:>8.3}",
            v[v.len() - 1], v[v.len() * 99 / 100], v[v.len() / 2]);
    };
    println!("\n  --- preview fidelity (export 60fps vs camera_track, same instants) ---");
    stat("|d cx| src px", &mut dcx);
    stat("|d cy| src px", &mut dcy);
    stat("|d scale|", &mut ds);
    stat("|d cx| screen", &mut dpx);
    stat("|d cursor| px", &mut dcur);
    println!("  worst |d cx| = {:.2} px at t={}ms (was 28.70 px at t=9344 on the old 16ms grid)", worst.1, worst.0);
    println!("  preview clock at t=12000ms: export sample #{} vs preview sample #{}",
        e.t.len(), p.t.len());
    assert_eq!(e.t.len(), p.t.len(), "the preview must sample the export's own frame grid");
    assert!(worst.1 < 1e-3, "preview and export diverged by {:.3}px at t={}ms", worst.1, worst.0);
}
