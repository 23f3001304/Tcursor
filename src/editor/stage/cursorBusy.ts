// Pack format v2's animated busy cursor, mirroring Rust `export/cursor/busy.rs` exactly. The
// export and the preview run the SAME math on the SAME output-clock timestamp, so a paused
// preview shows precisely the frame the export would write for that instant.
//
// `src/editor/stage/cursorBusy.test.ts` asserts the same five instants `busy_tests.rs` does; any
// change to one side has to be made on the other in the same edit.

/** One 1-second animation cycle. `spin` is the exception: its cycle is `24 / fps` seconds. */
const CYCLE_MS = 1000;
/** `flip` holds this fraction of its cycle, then turns over the rest. */
const FLIP_HOLD = 0.7;
/** `pulse`'s peak scale at mid-cycle. */
const PULSE_PEAK = 1.06;

/** How a pack animates its busy cursor from a single still. Wire form of Rust `BusyAnim`. */
export type BusyAnim = "spin" | "flip" | "pulse";

/** A pack's busy animation: the declared `anim`/`fps`, plus however many explicit `busy_NN.png`
 *  frames its folder ships (`0` = none, so `anim` drives the pose instead). */
export interface BusySpec { anim: BusyAnim; fps: number; frames: number }

/** What to draw for the busy cursor at one instant: which frame, and the transform to apply about
 *  the hotspot. `frame` is always 0 for a synthesised animation; `angleDeg`/`scale` are always the
 *  identity for an explicit-frame one. */
export interface BusyPose { frame: number; angleDeg: number; scale: number }

/** The still, untransformed pose - what a v1 pack (no `busy` block) always draws. */
export const STILL: BusyPose = { frame: 0, angleDeg: 0, scale: 1 };

/** Whether a pose needs a canvas transform at all. */
export function isIdentity(p: BusyPose): boolean {
  return p.angleDeg === 0 && p.scale === 1;
}

/** The busy pose at output time `tMs`. The TS mirror of Rust `busy_pose`. */
export function busyPose(spec: BusySpec, tMs: number): BusyPose {
  if (spec.frames > 0) {
    const fps = Math.max(spec.fps, 0.001);
    const i = Math.max(0, Math.floor((tMs / 1000) * fps)) % spec.frames;
    return { ...STILL, frame: i };
  }
  if (spec.anim === "spin") {
    // `fps` sets the cycle: a full turn takes 24/fps seconds, so the default 24 is one turn per
    // second. Clockwise, about the sprite's own hotspot.
    return { ...STILL, angleDeg: mod(360 * ((tMs / 1000) * spec.fps / 24), 360) };
  }
  if (spec.anim === "flip") return { ...STILL, angleDeg: flipAngle(tMs) };
  return { ...STILL, scale: pulseScale(tMs) };
}

/** `flip`: still for the first 70% of the cycle, then a cosine-eased half turn over the last 30%.
 *  The completed cycles are ACCUMULATED so it always turns the same way instead of snapping back
 *  at each boundary - an hourglass tips over, it does not rock. */
function flipAngle(tMs: number): number {
  const cycles = Math.floor(tMs / CYCLE_MS);
  const u = tMs / CYCLE_MS - cycles;
  const turned = u < FLIP_HOLD ? 0 : 180 * easeInOut((u - FLIP_HOLD) / (1 - FLIP_HOLD));
  return mod(cycles * 180 + turned, 360);
}

/** `pulse`: 1.0 at the cycle edges, `PULSE_PEAK` at its middle, on the same cosine family. */
function pulseScale(tMs: number): number {
  const u = tMs / CYCLE_MS;
  return 1 + (PULSE_PEAK - 1) * bump(u - Math.floor(u));
}

/** Cosine ease 0 to 1 across `u` in 0..1 (half a cosine period): no jerk at either end. */
function easeInOut(u: number): number {
  return (1 - Math.cos(Math.PI * clamp01(u))) / 2;
}

/** Cosine bump: 0 at both ends of `u` in 0..1, 1 at its middle (a full cosine period). */
function bump(u: number): number {
  return (1 - Math.cos(2 * Math.PI * clamp01(u))) / 2;
}

function clamp01(u: number): number {
  return Math.min(1, Math.max(0, u));
}

/** Euclidean remainder, matching Rust's `rem_euclid` for the negative times a scrub can produce. */
function mod(a: number, n: number): number {
  return ((a % n) + n) % n;
}
