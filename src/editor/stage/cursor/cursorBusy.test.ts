import { describe, it, expect } from "vitest";
import { busyPose, isIdentity, STILL, type BusyAnim, type BusySpec } from "./cursorBusy";

const INSTANTS = [0, 250, 500, 750, 1000];
const spec = (anim: BusyAnim, fps = 24): BusySpec => ({ anim, fps, frames: 0 });

describe("busyPose parity with Rust busy_pose", () => {
  it("spins once per second at the default fps", () => {
    const expected = [0, 90, 180, 270, 0];
    INSTANTS.forEach((t, i) => {
      const p = busyPose(spec("spin"), t);
      expect(p.angleDeg).toBeCloseTo(expected[i], 3);
      expect([p.frame, p.scale]).toEqual([0, 1]);
    });
  });

  it("lets fps set the spin cycle length", () => {
    expect(busyPose(spec("spin", 12), 500).angleDeg).toBeCloseTo(90, 3);
    expect(busyPose(spec("spin", 48), 250).angleDeg).toBeCloseTo(180, 3);
  });

  it("holds then turns, and never turns back", () => {
    const expected = [0, 0, 0, 12.0577, 180];
    INSTANTS.forEach((t, i) => {
      const p = busyPose(spec("flip"), t);
      expect(p.angleDeg).toBeCloseTo(expected[i], 3);
      expect([p.frame, p.scale]).toEqual([0, 1]);
    });
    expect(busyPose(spec("flip"), 900).angleDeg).toBeGreaterThan(12.06);
    expect(busyPose(spec("flip"), 2000).angleDeg).toBeCloseTo(0, 3);
  });

  it("breathes to its peak at mid-cycle and back", () => {
    const expected = [1, 1.03, 1.06, 1.03, 1];
    INSTANTS.forEach((t, i) => {
      const p = busyPose(spec("pulse"), t);
      expect(p.scale).toBeCloseTo(expected[i], 4);
      expect([p.frame, p.angleDeg]).toEqual([0, 0]);
    });
  });
});

describe("busyPose frames and edge cases", () => {
  it("lets explicit frames win over the synthesised animation", () => {
    const s: BusySpec = { anim: "spin", fps: 8, frames: 4 };
    for (const [t, want] of [
      [0, 0],
      [125, 1],
      [250, 2],
      [375, 3],
      [500, 0],
      [1000, 0],
    ]) {
      const p = busyPose(s, t);
      expect(p.frame).toBe(want);
      expect(isIdentity(p)).toBe(true);
    }
  });

  it("cannot divide by zero on a frame pack with no fps", () => {
    expect(busyPose({ anim: "spin", fps: 0, frames: 3 }, 10_000).frame).toBe(0);
  });

  it("reports the still pose, and a real one, for the transform gate", () => {
    expect(isIdentity(STILL)).toBe(true);
    expect(isIdentity(busyPose(spec("spin"), 0))).toBe(true);
    expect(isIdentity(busyPose(spec("spin"), 250))).toBe(false);
    expect(isIdentity(busyPose(spec("pulse"), 500))).toBe(false);
  });

  it("stays in range for a negative time, the way rem_euclid does", () => {
    const a = busyPose(spec("spin"), -250).angleDeg;
    expect(a).toBeGreaterThanOrEqual(0);
    expect(a).toBeLessThan(360);
  });
});
