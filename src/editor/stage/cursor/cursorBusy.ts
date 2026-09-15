const CYCLE_MS = 1000;

const FLIP_HOLD = 0.7;

const PULSE_PEAK = 1.06;

export type BusyAnim = "spin" | "flip" | "pulse";

export interface BusySpec {
  anim: BusyAnim;
  fps: number;
  frames: number;
}

export interface BusyPose {
  frame: number;
  angleDeg: number;
  scale: number;
}

export const STILL: BusyPose = { frame: 0, angleDeg: 0, scale: 1 };

export function isIdentity(p: BusyPose): boolean {
  return p.angleDeg === 0 && p.scale === 1;
}

export function busyPose(spec: BusySpec, tMs: number): BusyPose {
  if (spec.frames > 0) {
    const fps = Math.max(spec.fps, 0.001);
    const i = Math.max(0, Math.floor((tMs / 1000) * fps)) % spec.frames;
    return { ...STILL, frame: i };
  }
  if (spec.anim === "spin") {
    return { ...STILL, angleDeg: mod(360 * (((tMs / 1000) * spec.fps) / 24), 360) };
  }
  if (spec.anim === "flip") return { ...STILL, angleDeg: flipAngle(tMs) };
  return { ...STILL, scale: pulseScale(tMs) };
}

function flipAngle(tMs: number): number {
  const cycles = Math.floor(tMs / CYCLE_MS);
  const u = tMs / CYCLE_MS - cycles;
  const turned = u < FLIP_HOLD ? 0 : 180 * easeInOut((u - FLIP_HOLD) / (1 - FLIP_HOLD));
  return mod(cycles * 180 + turned, 360);
}

function pulseScale(tMs: number): number {
  const u = tMs / CYCLE_MS;
  return 1 + (PULSE_PEAK - 1) * bump(u - Math.floor(u));
}

function easeInOut(u: number): number {
  return (1 - Math.cos(Math.PI * clamp01(u))) / 2;
}

function bump(u: number): number {
  return (1 - Math.cos(2 * Math.PI * clamp01(u))) / 2;
}

function clamp01(u: number): number {
  return Math.min(1, Math.max(0, u));
}

function mod(a: number, n: number): number {
  return ((a % n) + n) % n;
}
