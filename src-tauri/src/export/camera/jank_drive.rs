use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Grid {
    Export,
    Preview,
}
impl Grid {
    pub fn t(self, i: u32) -> u32 {
        (i as u64 * 1000 / 60) as u32
    }
    pub fn dt(self) -> f32 {
        STEP_MS
    }
    pub fn name(self) -> &'static str {
        match self {
            Grid::Export => "export 60fps",
            Grid::Preview => "preview (camera_track)",
        }
    }
}

pub const STEP_MS: f32 = 1000.0 / 60.0;

pub struct Run {
    pub grid: Grid,
    pub t: Vec<u32>,
    pub scale: Vec<f32>,
    pub cx: Vec<f32>,
    pub cy: Vec<f32>,
    pub curx: Vec<f32>,
    pub cury: Vec<f32>,
    pub clamped: Vec<bool>,
    pub driver: Vec<i32>,
}

fn winner(rs: &[ZoomRegion], t: u32) -> i32 {
    rs.iter()
        .enumerate()
        .filter(|(_, r)| t >= r.start_ms && t <= r.end_ms)
        .max_by_key(|(i, r)| (r.layer, *i))
        .map(|(i, _)| i as i32)
        .unwrap_or(-1)
}

pub fn micro_region(start: u32, end: u32, follow: bool) -> ZoomRegion {
    ZoomRegion {
        start_ms: start,
        end_ms: end,
        zoom_in_ms: ZI,
        zoom_out_ms: ZO,
        target_scale: 2.2,
        anchor: FramePoint { x: 960, y: 540 },
        easing: Easing::Smooth,
        easing_out: Easing::Smooth,
        layer: 0,
        cam_action: None,
        follow_cursor: follow,
    }
}

pub fn drive(
    rs: &[ZoomRegion],
    cfg: &ZoomConfig,
    t0: u32,
    t1: u32,
    cur: impl Fn(u32) -> FramePoint,
) -> (Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut s = CameraSim::new(FW, FH);
    let (mut ts, mut xs, mut ys, mut ss) = (vec![], vec![], vec![], vec![]);
    for i in 0.. {
        let t = (i as u64 * 1000 / 60) as u32;
        if t > t1 {
            break;
        }
        let c = s.step(t, STEP_MS, cur(t), rs, cfg);
        if t >= t0 {
            ts.push(t);
            xs.push(c.cx);
            ys.push(c.cy);
            ss.push(c.scale);
        }
    }
    (ts, xs, ys, ss)
}

pub fn vel(x: &[f32]) -> Vec<f32> {
    (0..x.len())
        .map(|i| if i == 0 { 0.0 } else { x[i] - x[i - 1] })
        .collect()
}

pub fn band(ts: &[u32], v: &[f32], a: u32, b: u32) -> f32 {
    let s: Vec<f32> = ts
        .iter()
        .zip(v)
        .filter(|(t, _)| **t >= a && **t <= b)
        .map(|(_, x)| *x)
        .collect();
    if s.is_empty() {
        0.0
    } else {
        s.iter().map(|x| x.abs()).sum::<f32>() / s.len() as f32
    }
}

pub fn run(grid: Grid, cfg: &ZoomConfig, smoothness: f32) -> Run {
    let (rs, mut cur) = (regions(), Cursor::new(events(), screen(), smoothness));
    let mut sim = CameraSim::new(FW, FH);
    let mut r = Run {
        grid,
        t: vec![],
        scale: vec![],
        cx: vec![],
        cy: vec![],
        curx: vec![],
        cury: vec![],
        clamped: vec![],
        driver: vec![],
    };
    let mut i = 0u32;
    loop {
        let t = grid.t(i);
        if t > DUR_MS {
            break;
        }
        let p = cur.at(t, grid.dt());
        let c = sim.step(t, grid.dt(), p, &rs, cfg);
        let (hw, hh) = (
            FW as f32 / (2.0 * c.scale.max(0.01)),
            FH as f32 / (2.0 * c.scale.max(0.01)),
        );
        let at_edge = (c.cx - hw).abs() < 1e-3
            || (c.cx - (FW as f32 - hw)).abs() < 1e-3
            || (c.cy - hh).abs() < 1e-3
            || (c.cy - (FH as f32 - hh)).abs() < 1e-3;
        r.t.push(t);
        r.scale.push(c.scale);
        r.cx.push(c.cx);
        r.cy.push(c.cy);
        r.curx.push(p.x as f32);
        r.cury.push(p.y as f32);
        r.clamped.push(at_edge);
        r.driver.push(winner(&rs, t));
        i += 1;
    }
    r
}
