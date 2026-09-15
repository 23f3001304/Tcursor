export const REF_W = 1920;
const SUB_MS = 1;
const REF_STEP_MS = 1000 / 60;
const VEL_A = 0.35;
const K = 2;
const DEAD = 0.4;

export const MAX_DEG = 6;
const W = 18,
  Z = 0.5;
const MAX_CATCHUP_MS = 100;

export interface TiltState {
  vx: number;
  vy: number;
  angle: number;
  avel: number;
  px: number;
  py: number;
  acc: number;
  primed: boolean;
}

export function newTilt(): TiltState {
  return { vx: 0, vy: 0, angle: 0, avel: 0, px: 0, py: 0, acc: 0, primed: false };
}

export function resetTilt(s: TiltState) {
  s.vx = 0;
  s.vy = 0;
  s.angle = 0;
  s.avel = 0;
  s.px = 0;
  s.py = 0;
  s.acc = 0;
  s.primed = false;
}

function targetDeg(vx: number, vy: number, max: number): number {
  const speed = Math.hypot(vx, vy);
  if (speed <= DEAD) return 0;
  const gate = (speed - DEAD) / speed;
  return Math.min(max, Math.max(-max, K * (vx + 0.5 * vy) * gate));
}

export function tiltStep(s: TiltState, x: number, y: number, dtMs: number, maxDeg: number): number {
  if (!s.primed || !(dtMs > 0)) {
    s.px = x;
    s.py = y;
    s.primed = true;
    return s.angle;
  }
  const ivx = (x - s.px) / dtMs,
    ivy = (y - s.py) / dtMs;
  s.px = x;
  s.py = y;
  s.acc = Math.min(s.acc + dtMs, MAX_CATCHUP_MS);
  const a = 1 - Math.pow(1 - VEL_A, SUB_MS / REF_STEP_MS);
  const h = SUB_MS / 1000;
  const max = Math.max(maxDeg, 0);
  while (s.acc >= SUB_MS) {
    s.acc -= SUB_MS;
    s.vx += (ivx - s.vx) * a;
    s.vy += (ivy - s.vy) * a;
    const target = targetDeg(s.vx, s.vy, max);
    s.avel += (-W * W * (s.angle - target) - 2 * Z * W * s.avel) * h;
    s.angle += s.avel * h;
  }
  return s.angle;
}

export function maxTiltDeg(tilt: number): number {
  return tilt > 0 ? MAX_DEG * Math.min(1, tilt) : 0;
}

export function tiltFromCam(
  s: TiltState,
  curx: number,
  cury: number,
  aspect: number,
  dtMs: number,
  tilt: number,
): number {
  const max = maxTiltDeg(tilt);
  if (max <= 0) {
    if (s.angle !== 0 || s.primed) resetTilt(s);
    return 0;
  }
  return tiltStep(s, curx * REF_W, (cury * REF_W) / Math.max(aspect, 1e-6), dtMs, max);
}
