use crate::export::types::Camera;

const SETTLE: f32 = 5.0;

#[derive(Clone, Copy, Default)]
struct Chan {
    y: f32,
    v: f32,
}

impl Chan {
    fn step(&mut self, x: f32, w: f32, dt: f32) -> f32 {
        let d = self.y - x;
        let b = self.v + w * d;
        let e = (-w * dt).exp();
        self.y = x + (d + b * dt) * e;
        self.v = (self.v - w * b * dt) * e;
        self.y
    }
}

pub(crate) struct Damped2 {
    s: Chan,
    x: Chan,
    y: Chan,
    on: bool,
}

impl Damped2 {
    pub(crate) fn new() -> Self {
        Self {
            s: Chan::default(),
            x: Chan::default(),
            y: Chan::default(),
            on: false,
        }
    }

    pub(crate) fn apply(&mut self, dt_ms: f32, cam: Camera, ms: u32, fw: f32, fh: f32) -> Camera {
        if ms == 0 {
            self.on = false;
            return cam;
        }
        if !self.on {
            self.on = true;
            self.s = Chan {
                y: cam.scale,
                v: 0.0,
            };
            self.x = Chan { y: cam.cx, v: 0.0 };
            self.y = Chan { y: cam.cy, v: 0.0 };
            return cam;
        }
        let dt = dt_ms / 1000.0;
        let w = SETTLE / (ms as f32 / 1000.0);
        let scale = self.s.step(cam.scale, w, dt).max(0.01);
        let (cx, cy) = (self.x.step(cam.cx, w, dt), self.y.step(cam.cy, w, dt));
        let (hw, hh) = (fw / (2.0 * scale), fh / (2.0 * scale));
        Camera {
            cx: cx.clamp(hw, (fw - hw).max(hw)),
            cy: cy.clamp(hh, (fh - hh).max(hh)),
            scale,
        }
    }
}
