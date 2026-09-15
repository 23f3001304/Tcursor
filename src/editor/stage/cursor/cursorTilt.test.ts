import { describe, it, expect } from "vitest";
import { maxTiltDeg, newTilt, resetTilt, tiltFromCam, tiltStep, MAX_DEG, REF_W } from "./cursorTilt";

const STEP60 = 16.666666030883789;

function throwRun(dt: number, speed: number, moveMs: number, endMs: number, max: number) {
  const s = newTilt();
  const n = Math.round(endMs / dt);
  const out: [number, number][] = [];
  for (let i = 0; i <= n; i++) {
    const t = i * dt;
    out.push([t, tiltStep(s, speed * Math.min(t, moveMs), 0, dt, max)]);
  }
  return out;
}
const at = (r: [number, number][], t: number) =>
  r.reduce((b, p) => (Math.abs(p[0] - t) < Math.abs(b[0] - t) ? p : b))[1];

describe("the cursor's motion tilt", () => {
  it("matches the five pins Rust's tilt_tests.rs asserts", () => {
    const r = throwRun(STEP60, 2.0, 200, 600, MAX_DEG);
    for (const [t, want] of [
      [100, 1.213484],
      [200, 3.291586],
      [300, 1.707726],
      [400, -0.444513],
      [600, 0.070651],
    ]) {
      expect(at(r, t)).toBeCloseTo(want, 3);
    }
  });

  it("leans into a fast throw and never runs away past the cap", () => {
    expect(at(throwRun(STEP60, 2.0, 600, 600, MAX_DEG), 500)).toBeGreaterThan(1);
    for (const [tilt, speed] of [
      [1, 8],
      [0.35, 8],
      [0.35, 2],
    ]) {
      const max = maxTiltDeg(tilt);
      for (const [, a] of throwRun(STEP60, speed, 600, 900, max)) expect(a).toBeLessThanOrEqual(max * 1.2);
      const held = throwRun(STEP60, speed, 1500, 1500, max);
      expect(held[held.length - 1][1]).toBeCloseTo(max, 2);
    }
  });

  it("comes back upright through exactly one overshoot when the cursor stops", () => {
    const r = throwRun(STEP60, 2.5, 400, 1200, MAX_DEG).filter(([t]) => t >= 400);
    const lows = r.filter(
      ([, a], i) => i > 0 && i < r.length - 1 && a < -0.2 && a <= r[i - 1][1] && a <= r[i + 1][1],
    );
    expect(lows).toHaveLength(1);
    expect(lows[0][1]).toBeGreaterThan(-1.5);
    expect(Math.abs(at(r, 1000))).toBeLessThan(0.05);
  });

  it("stays upright for ordinary pointing, and at tilt 0", () => {
    for (const speed of [0.05, 0.2, 0.39]) {
      for (const [, a] of throwRun(STEP60, speed, 2000, 2000, MAX_DEG))
        expect(Math.abs(a)).toBeLessThan(1e-9);
    }
    for (const [, a] of throwRun(STEP60, 8, 600, 1200, maxTiltDeg(0))) expect(a).toBe(0);
    expect(maxTiltDeg(0)).toBe(0);
    expect(maxTiltDeg(2)).toBe(MAX_DEG);
    expect(maxTiltDeg(NaN)).toBe(0);
  });

  it("means the same thing at 30fps as at 60fps", () => {
    const a = throwRun(STEP60, 2.5, 400, 1200, MAX_DEG),
      b = throwRun(2 * STEP60, 2.5, 400, 1200, MAX_DEG);
    for (let k = 1; k <= 12; k++) expect(at(a, k * 100)).toBeCloseTo(at(b, k * 100), 3);
  });

  it("only records the position on a tick with no elapsed time", () => {
    const s = newTilt();
    tiltStep(s, 0, 0, STEP60, MAX_DEG);
    expect(tiltStep(s, 200, 0, 0, MAX_DEG)).toBe(0);
    expect(tiltStep(s, 200, 0, STEP60, MAX_DEG)).toBe(0);
    resetTilt(s);
    expect(s.primed).toBe(false);
  });
});

describe("tiltFromCam", () => {
  it("reads the camera track's fractions as reference pixels", () => {
    const s = newTilt(),
      direct = newTilt();
    for (let i = 0; i <= 24; i++) {
      const t = i * STEP60,
        x = 2 * Math.min(t, 200);
      tiltFromCam(s, x / REF_W, 0.5, 16 / 9, STEP60, 1);
      tiltStep(direct, x, (0.5 * REF_W) / (16 / 9), STEP60, MAX_DEG);
    }
    expect(s.angle).toBeCloseTo(direct.angle, 9);
  });

  it("is a no-op at tilt 0 and unwinds a lean the setting just switched off", () => {
    const s = newTilt();
    for (let i = 0; i <= 24; i++) tiltFromCam(s, (2 * i * STEP60) / REF_W, 0, 16 / 9, STEP60, 1);
    expect(s.angle).toBeGreaterThan(1);
    expect(tiltFromCam(s, 0.9, 0, 16 / 9, STEP60, 0)).toBe(0);
    expect(s.angle).toBe(0);
  });

  it("leans a vertical throw at half the weight of a horizontal one", () => {
    const down = newTilt(),
      right = newTilt();
    for (let i = 0; i <= 36; i++) {
      const t = i * STEP60;
      tiltFromCam(down, 0.5, (2 * t * (16 / 9)) / REF_W, 16 / 9, STEP60, 1);
      tiltFromCam(right, (2 * t) / REF_W, 0.5, 16 / 9, STEP60, 1);
    }
    expect(down.angle / right.angle).toBeCloseTo(0.5, 2);
  });
});
