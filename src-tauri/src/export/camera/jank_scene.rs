use crate::events::model::{Button, EventKind, MouseEvent, ScreenInfo};
use crate::export::camera::CameraSim;
use crate::export::cursor::Cursor;
use crate::export::types::{Easing, FramePoint, ZoomConfig, ZoomRegion};

pub const FW: u32 = 1920;
pub const FH: u32 = 1080;
pub const DUR_MS: u32 = 12_000;

pub const SWEEP: (u32, u32) = (3000, 3400);

pub const SMOOTH_DEFAULT: f32 = 0.6;

const WAY: [(u32, f32, f32); 11] = [
    (0, 600.0, 500.0),
    (900, 640.0, 520.0),
    (1400, 650.0, 530.0),
    (2600, 300.0, 500.0),
    (3000, 300.0, 500.0),
    (3400, 1700.0, 800.0),
    (3600, 1700.0, 800.0),
    (6000, 1690.0, 795.0),
    (7000, 900.0, 400.0),
    (9000, 950.0, 420.0),
    (12_000, 1000.0, 450.0),
];

pub const R1: (u32, u32) = (2600, 5000);
pub const R2: (u32, u32) = (5000, 7500);
pub const R3: (u32, u32) = (9000, 12_000);
pub const ZI: u32 = 350;
pub const ZO: u32 = 450;

pub fn screen() -> ScreenInfo {
    ScreenInfo {
        w: FW,
        h: FH,
        origin_x: 0,
        origin_y: 0,
    }
}

fn path(t: u32) -> (f32, f32) {
    let mut i = 0;
    while i + 1 < WAY.len() && WAY[i + 1].0 <= t {
        i += 1;
    }
    let (t0, x0, y0) = WAY[i];
    match WAY.get(i + 1) {
        Some(&(t1, x1, y1)) => {
            let f = (t - t0) as f32 / (t1 - t0).max(1) as f32;
            (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f)
        }
        None => (x0, y0),
    }
}

fn jitter(i: u32) -> (f32, f32) {
    let h = i.wrapping_mul(2_654_435_761).rotate_left(13) ^ 0x9E37_79B9;
    (((h >> 5) % 5) as f32 - 2.0, ((h >> 17) % 5) as f32 - 2.0)
}

pub fn events() -> Vec<MouseEvent> {
    let mut out = Vec::new();
    let (mut t, mut i) = (0u32, 0u32);
    while t <= DUR_MS {
        let ((px, py), (jx, jy)) = (path(t), jitter(i));
        out.push(MouseEvent {
            t,
            kind: EventKind::Move,
            x: (px + jx) as i32,
            y: (py + jy) as i32,
            button: None,
        });
        t += 8;
        i += 1;
    }
    for &ct in &[1000u32, 1400] {
        let (x, y) = path(ct);
        let (x, y) = (x as i32, y as i32);
        out.push(MouseEvent {
            t: ct,
            kind: EventKind::Down,
            x,
            y,
            button: Some(Button::Left),
        });
        out.push(MouseEvent {
            t: ct + 60,
            kind: EventKind::Up,
            x,
            y,
            button: Some(Button::Left),
        });
    }
    out.sort_by_key(|e| e.t);
    out
}

pub fn regions() -> Vec<ZoomRegion> {
    let base = ZoomRegion {
        start_ms: 0,
        end_ms: 0,
        zoom_in_ms: ZI,
        zoom_out_ms: ZO,
        target_scale: 2.2,
        anchor: FramePoint { x: 960, y: 540 },
        easing: Easing::Smooth,
        easing_out: Easing::Smooth,
        layer: 0,
        cam_action: None,
        follow_cursor: false,
    };
    vec![
        ZoomRegion {
            start_ms: R1.0,
            end_ms: R1.1,
            follow_cursor: true,
            ..base
        },
        ZoomRegion {
            start_ms: R2.0,
            end_ms: R2.1,
            target_scale: 1.8,
            anchor: FramePoint { x: 1400, y: 760 },
            ..base
        },
        ZoomRegion {
            start_ms: R3.0,
            end_ms: R3.1,
            target_scale: 2.6,
            anchor: FramePoint { x: 1500, y: 1000 },
            ..base
        },
    ]
}

#[path = "jank_drive.rs"]
mod harness;
pub use harness::*;
