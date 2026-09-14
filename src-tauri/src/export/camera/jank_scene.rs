// Synthetic 12s scenario for the camera jank probe (`jank_probe_tests.rs`): a hand-made mouse
// track (8ms samples + jitter, two clicks, one fast sweep, a pause) and three zoom regions,
// driven straight through `Cursor` + `CameraSim` with NO renderer, so every number the probe
// prints is the camera math alone (no decode, no layout, no compositor).
use crate::events::model::{Button, EventKind, MouseEvent, ScreenInfo};
use crate::export::camera::CameraSim;
use crate::export::cursor::Cursor;
use crate::export::types::{Easing, FramePoint, ZoomConfig, ZoomRegion};

pub const FW: u32 = 1920;
pub const FH: u32 = 1080;
pub const DUR_MS: u32 = 12_000;
/// The fast sweep's window - where "does the camera track?" and the filter's lag are measured.
pub const SWEEP: (u32, u32) = (3000, 3400);
/// The default `CursorSettings::smoothness` - the glide strength `Cursor` draws with.
pub const SMOOTH_DEFAULT: f32 = 0.6;

/// (t_ms, x, y) waypoints of the hand-made path, linearly interpolated between: a slow drift
/// with two clicks, a move to the left edge, a 400ms 1400px sweep, then a long near-still pause.
const WAY: [(u32, f32, f32); 11] = [
    (0, 600.0, 500.0), (900, 640.0, 520.0), (1400, 650.0, 530.0), (2600, 300.0, 500.0),
    (3000, 300.0, 500.0), (3400, 1700.0, 800.0), (3600, 1700.0, 800.0), (6000, 1690.0, 795.0),
    (7000, 900.0, 400.0), (9000, 950.0, 420.0), (12_000, 1000.0, 450.0),
];

/// Region timetable, mirrored as consts so the spike classifier can name a phase boundary
/// without re-deriving it: (start_ms, end_ms) and the fitted ramp ends.
pub const R1: (u32, u32) = (2600, 5000); // follow_cursor 2.2, 350 in / 450 out -> hold 2950..4550
pub const R2: (u32, u32) = (5000, 7500); // anchored 1.8, gapless after R1
pub const R3: (u32, u32) = (9000, 12_000); // anchored 2.6 at a corner -> the in-frame clamp bites
pub const ZI: u32 = 350;
pub const ZO: u32 = 450;

pub fn screen() -> ScreenInfo { ScreenInfo { w: FW, h: FH, origin_x: 0, origin_y: 0 } }

fn path(t: u32) -> (f32, f32) {
    let mut i = 0;
    while i + 1 < WAY.len() && WAY[i + 1].0 <= t { i += 1; }
    let (t0, x0, y0) = WAY[i];
    match WAY.get(i + 1) {
        Some(&(t1, x1, y1)) => {
            let f = (t - t0) as f32 / (t1 - t0).max(1) as f32;
            (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f)
        }
        None => (x0, y0),
    }
}

/// Deterministic +-2px per-sample jitter - real `WH_MOUSE_LL` samples are never on a clean line.
fn jitter(i: u32) -> (f32, f32) {
    let h = i.wrapping_mul(2_654_435_761).rotate_left(13) ^ 0x9E37_79B9;
    (((h >> 5) % 5) as f32 - 2.0, ((h >> 17) % 5) as f32 - 2.0)
}

pub fn events() -> Vec<MouseEvent> {
    let mut out = Vec::new();
    let (mut t, mut i) = (0u32, 0u32);
    while t <= DUR_MS {
        let ((px, py), (jx, jy)) = (path(t), jitter(i));
        out.push(MouseEvent { t, kind: EventKind::Move, x: (px + jx) as i32, y: (py + jy) as i32, button: None });
        t += 8;
        i += 1;
    }
    for &ct in &[1000u32, 1400] {
        let (x, y) = path(ct);
        let (x, y) = (x as i32, y as i32);
        out.push(MouseEvent { t: ct, kind: EventKind::Down, x, y, button: Some(Button::Left) });
        out.push(MouseEvent { t: ct + 60, kind: EventKind::Up, x, y, button: Some(Button::Left) });
    }
    out.sort_by_key(|e| e.t);
    out
}

pub fn regions() -> Vec<ZoomRegion> {
    let base = ZoomRegion { start_ms: 0, end_ms: 0, zoom_in_ms: ZI, zoom_out_ms: ZO, target_scale: 2.2,
        anchor: FramePoint { x: 960, y: 540 }, easing: Easing::Smooth, layer: 0, cam_action: None,
        follow_cursor: false };
    vec![
        ZoomRegion { start_ms: R1.0, end_ms: R1.1, follow_cursor: true, ..base },
        ZoomRegion { start_ms: R2.0, end_ms: R2.1, target_scale: 1.8, anchor: FramePoint { x: 1400, y: 760 }, ..base },
        ZoomRegion { start_ms: R3.0, end_ms: R3.1, target_scale: 2.6, anchor: FramePoint { x: 1500, y: 1000 }, ..base },
    ]
}

/// The two sample grids the same camera math is driven on: the exporter's own
/// `t = k * 1000 / out_fps` (integer division, so the TIMESTAMPS land 16/17ms apart even though
/// the frames are a uniform 16.667ms) and the editor preview's (`preview_track::camera_track`).
/// The preview used to step a flat integer 16ms - 4% more steps per second than the export ever
/// takes - and now walks the export's own frame index, so the two are the same grid.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Grid { Export, Preview }
impl Grid {
    pub fn t(self, i: u32) -> u32 { (i as u64 * 1000 / 60) as u32 }
    /// The EXACT frame period handed to `CameraSim::step` - never `t(i) - t(i-1)`.
    pub fn dt(self) -> f32 { STEP_MS }
    pub fn name(self) -> &'static str {
        match self { Grid::Export => "export 60fps", Grid::Preview => "preview (camera_track)" }
    }
}

/// `1000 / 60` exactly - what `render::OUT_STEP_MS` hands the sim on every 60fps path.
pub const STEP_MS: f32 = 1000.0 / 60.0;

/// One sampled trajectory: everything the metrics module needs, per step.
pub struct Run {
    pub grid: Grid,
    pub t: Vec<u32>,
    pub scale: Vec<f32>, pub cx: Vec<f32>, pub cy: Vec<f32>,
    pub curx: Vec<f32>, pub cury: Vec<f32>,
    pub clamped: Vec<bool>, // the in-frame clamp actually moved cx/cy this step
    pub driver: Vec<i32>,   // winning region index, -1 for none (mirrors `CameraSim::winner`)
}

/// `CameraSim::winner`'s rule, mirrored (it is private) so a spike can be attributed to a handoff.
fn winner(rs: &[ZoomRegion], t: u32) -> i32 {
    rs.iter().enumerate().filter(|(_, r)| t >= r.start_ms && t <= r.end_ms)
        .max_by_key(|(i, r)| (r.layer, *i)).map(|(i, _)| i as i32).unwrap_or(-1)
}

/// A bare region for the per-hypothesis micro-probes: 2.2x, the scene's own ramp durations.
pub fn micro_region(start: u32, end: u32, follow: bool) -> ZoomRegion {
    ZoomRegion { start_ms: start, end_ms: end, zoom_in_ms: ZI, zoom_out_ms: ZO, target_scale: 2.2,
        anchor: FramePoint { x: 960, y: 540 }, easing: Easing::Smooth, layer: 0, cam_action: None,
        follow_cursor: follow }
}

/// Drive a fresh sim over `[0, t1]` on the exporter's grid with a hand-written cursor, keeping
/// samples from `t0`: `(t, cx, cy, scale)`. The micro-probes' equivalent of `run`.
pub fn drive(rs: &[ZoomRegion], cfg: &ZoomConfig, t0: u32, t1: u32, cur: impl Fn(u32) -> FramePoint)
    -> (Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut s = CameraSim::new(FW, FH);
    let (mut ts, mut xs, mut ys, mut ss) = (vec![], vec![], vec![], vec![]);
    for i in 0.. {
        let t = (i as u64 * 1000 / 60) as u32;
        if t > t1 { break; }
        let c = s.step(t, STEP_MS, cur(t), rs, cfg);
        if t >= t0 { ts.push(t); xs.push(c.cx); ys.push(c.cy); ss.push(c.scale); }
    }
    (ts, xs, ys, ss)
}

/// First difference (px per frame), index-aligned with `x` (`[0]` = 0).
pub fn vel(x: &[f32]) -> Vec<f32> {
    (0..x.len()).map(|i| if i == 0 { 0.0 } else { x[i] - x[i - 1] }).collect()
}

/// Mean |v| over the samples whose time falls in `[a, b]` ms.
pub fn band(ts: &[u32], v: &[f32], a: u32, b: u32) -> f32 {
    let s: Vec<f32> = ts.iter().zip(v).filter(|(t, _)| **t >= a && **t <= b).map(|(_, x)| *x).collect();
    if s.is_empty() { 0.0 } else { s.iter().map(|x| x.abs()).sum::<f32>() / s.len() as f32 }
}

/// Drive `Cursor::at` + `CameraSim::step` over the whole clip on `grid`, recording every sample.
/// `smoothness` is the cursor glide strength (`CursorSettings::smoothness`).
pub fn run(grid: Grid, cfg: &ZoomConfig, smoothness: f32) -> Run {
    let (rs, mut cur) = (regions(), Cursor::new(events(), screen(), smoothness));
    let mut sim = CameraSim::new(FW, FH);
    let mut r = Run { grid, t: vec![], scale: vec![], cx: vec![], cy: vec![], curx: vec![], cury: vec![],
        clamped: vec![], driver: vec![] };
    let mut i = 0u32;
    loop {
        let t = grid.t(i);
        if t > DUR_MS { break; }
        let p = cur.at(t, grid.dt());
        let c = sim.step(t, grid.dt(), p, &rs, cfg);
        let (hw, hh) = (FW as f32 / (2.0 * c.scale.max(0.01)), FH as f32 / (2.0 * c.scale.max(0.01)));
        let at_edge = (c.cx - hw).abs() < 1e-3 || (c.cx - (FW as f32 - hw)).abs() < 1e-3
            || (c.cy - hh).abs() < 1e-3 || (c.cy - (FH as f32 - hh)).abs() < 1e-3;
        r.t.push(t); r.scale.push(c.scale); r.cx.push(c.cx); r.cy.push(c.cy);
        r.curx.push(p.x as f32); r.cury.push(p.y as f32);
        r.clamped.push(at_edge); r.driver.push(winner(&rs, t));
        i += 1;
    }
    r
}
