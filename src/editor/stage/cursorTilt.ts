// The live preview's mirror of Rust `export/cursor/tilt.rs` - the very slight lean a cursor takes
// on when it is thrown across the screen, and the single overshoot it corrects through when it
// stops. Same two filters in series (a low-pass on the cursor's own velocity, then a lightly
// under-damped spring chasing the angle that velocity asks for), same constants, same fixed 1 ms
// substep grid, and `cursorTilt.test.ts` pins the two against the same five instants.
//
// Rust computes this in f64 for exactly this reason: the preview must not lean differently from
// the frame the backend hands back on pause, or pausing would visibly twitch the cursor.

/** The screen width every tilt speed is measured against, so one gesture leans the same whether
 *  the recording is 1080p or 4K (Rust `tilt::REF_W`). The preview works in this unit directly:
 *  `CamSample.curx` is already a fraction of the screen content, so it never needs to know the
 *  recording's pixel size. */
export const REF_W = 1920;
const SUB_MS = 1;
const REF_STEP_MS = 1000 / 60;
const VEL_A = 0.35;
const K = 2;
const DEAD = 0.4;
/** The lean a full-speed sweep settles at when `tilt` is 1, in degrees (Rust `tilt::MAX_DEG`). */
export const MAX_DEG = 6;
const W = 18, Z = 0.5;
const MAX_CATCHUP_MS = 100;

/** The lean filter's state - one per stage, reset on a seek the way the motion trail is. */
export interface TiltState {
  vx: number; vy: number; angle: number; avel: number;
  px: number; py: number; acc: number; primed: boolean;
}

export function newTilt(): TiltState {
  return { vx: 0, vy: 0, angle: 0, avel: 0, px: 0, py: 0, acc: 0, primed: false };
}

/** Back to upright and unprimed (Rust `Tilt::reset`) - what a seek or a cut calls, so the cursor
 *  never arrives somewhere else still leaning from the gesture before it. */
export function resetTilt(s: TiltState) {
  s.vx = 0; s.vy = 0; s.angle = 0; s.avel = 0; s.px = 0; s.py = 0; s.acc = 0; s.primed = false;
}

/** The angle a velocity of `(vx, vy)` reference px/ms asks for, capped at `max`. The sign follows
 *  the HORIZONTAL direction of travel; a purely vertical throw leans by its vertical component at
 *  half weight. `gate` fades the lean in from the dead-zone edge so a cursor hovering right at the
 *  threshold does not flicker. Mirrors Rust `tilt::target_deg`. */
function targetDeg(vx: number, vy: number, max: number): number {
  const speed = Math.hypot(vx, vy);
  if (speed <= DEAD) return 0;
  const gate = (speed - DEAD) / speed;
  return Math.min(max, Math.max(-max, K * (vx + 0.5 * vy) * gate));
}

/** Advance by one frame of `dtMs` with the cursor at `(x, y)` in REFERENCE px, and return the new
 *  lean in degrees (clockwise-positive, the sense a canvas `rotate` turns in). `maxDeg` is the cap
 *  this frame's `tilt` setting allows (`maxTiltDeg`). The first call, and any non-positive `dtMs`
 *  (a paused redraw), only records the position: there is no velocity to read yet.
 *  Mirrors Rust `Tilt::step` step for step, including the substep carry. */
export function tiltStep(s: TiltState, x: number, y: number, dtMs: number, maxDeg: number): number {
  if (!s.primed || !(dtMs > 0)) { s.px = x; s.py = y; s.primed = true; return s.angle; }
  const ivx = (x - s.px) / dtMs, ivy = (y - s.py) / dtMs;
  s.px = x; s.py = y;
  s.acc = Math.min(s.acc + dtMs, MAX_CATCHUP_MS);
  const a = 1 - Math.pow(1 - VEL_A, SUB_MS / REF_STEP_MS);
  const h = SUB_MS / 1000;
  const max = Math.max(maxDeg, 0);
  while (s.acc >= SUB_MS) {
    s.acc -= SUB_MS;
    s.vx += (ivx - s.vx) * a;
    s.vy += (ivy - s.vy) * a;
    const target = targetDeg(s.vx, s.vy, max);
    // Semi-implicit Euler, like Rust: the velocity updates first and the angle integrates the NEW
    // velocity, which is what keeps a lightly-damped spring from gaining energy step over step.
    s.avel += (-W * W * (s.angle - target) - 2 * Z * W * s.avel) * h;
    s.angle += s.avel * h;
  }
  return s.angle;
}

/** The cap the `tilt` setting (0..1) allows, mirroring Rust `tilt::max_deg`. Written as a
 *  `> 0` test rather than `Math.max(0, tilt)` so a missing or NaN setting reads as OFF: a NaN cap
 *  would poison the spring's state for the rest of the session, where Rust's serde default cannot
 *  hand one over in the first place. */
export function maxTiltDeg(tilt: number): number {
  return tilt > 0 ? MAX_DEG * Math.min(1, tilt) : 0;
}

/** One preview tick's lean, from the camera track's own cursor fractions.
 *
 *  `curx`/`cury` are `CamSample`'s 0..1 position across the SCREEN CONTENT - the same numbers Rust
 *  derives from the smoothed cursor it feeds its own filter - and `aspect` is that content's width
 *  over its height, which is what turns the vertical fraction into the same reference-px unit as
 *  the horizontal one. `tilt` is the setting; 0 returns 0 without touching the filter. */
export function tiltFromCam(s: TiltState, curx: number, cury: number, aspect: number,
  dtMs: number, tilt: number): number {
  const max = maxTiltDeg(tilt);
  if (max <= 0) { if (s.angle !== 0 || s.primed) resetTilt(s); return 0; }
  return tiltStep(s, curx * REF_W, (cury * REF_W) / Math.max(aspect, 1e-6), dtMs, max);
}
