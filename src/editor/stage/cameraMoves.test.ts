import { describe, expect, it } from "vitest";
import type { CameraMove } from "../../lib/edit";
import { ease } from "../timeline/layoutTrack";
import { camMoveAt } from "./cameraMoves";

// Mirrors src-tauri/src/export/camera/moves_tests.rs - the Rust authority's cases,
// re-asserted in TS so the two never silently diverge.
const kf = (t_ms: number, x: number, y: number, size: number, easing: string): CameraMove =>
  ({ id: "k", t_ms, x, y, size, easing });

describe("camMoveAt", () => {
  it("empty track samples to null", () => {
    expect(camMoveAt([], 0)).toBeNull();
    expect(camMoveAt([], 5000)).toBeNull();
  });

  it("single keyframe holds at any time", () => {
    const moves = [kf(1000, 0.3, 0.7, 0.2, "smooth")];
    const want = { x: 0.3, y: 0.7, size: 0.2 };
    expect(camMoveAt(moves, 0)).toEqual(want);
    expect(camMoveAt(moves, 1000)).toEqual(want);
    expect(camMoveAt(moves, 50_000)).toEqual(want);
  });

  it("holds first pose before and at first keyframe", () => {
    const moves = [kf(1000, 0.1, 0.1, 0.1, "linear"), kf(2000, 0.9, 0.9, 0.5, "linear")];
    const first = { x: 0.1, y: 0.1, size: 0.1 };
    expect(camMoveAt(moves, 0)).toEqual(first);
    expect(camMoveAt(moves, 999)).toEqual(first);
    expect(camMoveAt(moves, 1000)).toEqual(first);
  });

  it("holds last pose at and after last keyframe", () => {
    const moves = [kf(1000, 0.1, 0.1, 0.1, "linear"), kf(2000, 0.9, 0.9, 0.5, "linear")];
    const last = { x: 0.9, y: 0.9, size: 0.5 };
    expect(camMoveAt(moves, 2000)).toEqual(last);
    expect(camMoveAt(moves, 9000)).toEqual(last);
  });

  it("midpoint lerp with linear easing is the exact mean", () => {
    const moves = [kf(0, 0.0, 0.0, 0.0, "linear"), kf(1000, 1.0, 1.0, 1.0, "linear")];
    const p = camMoveAt(moves, 500)!;
    expect(p.x).toBeCloseTo(0.5, 6);
    expect(p.y).toBeCloseTo(0.5, 6);
    expect(p.size).toBeCloseTo(0.5, 6);
  });

  it("midpoint with smooth easing diverges from the linear mean", () => {
    // smoothstep(0.5) == 0.5 exactly, so a symmetric pair alone wouldn't distinguish
    // smooth from linear - use asymmetric per-axis deltas, assert against ease() directly.
    const moves = [kf(0, 0.0, 0.2, 0.1, "smooth"), kf(1000, 1.0, 0.2, 0.9, "smooth")];
    const f = ease("smooth", 0.25); // != 0.25 for smoothstep away from the midpoint
    expect(Math.abs(f - 0.25)).toBeGreaterThan(1e-6);
    const p = camMoveAt(moves, 250)!;
    expect(p.x).toBeCloseTo(f, 6);
    expect(p.size).toBeCloseTo(0.1 + 0.8 * f, 6);
  });

  it("coincident keyframe times snap to b without dividing by zero", () => {
    const moves = [kf(1000, 0.2, 0.2, 0.2, "linear"), kf(1000, 0.8, 0.8, 0.8, "linear"), kf(2000, 0.0, 0.0, 0.0, "linear")];
    const p = camMoveAt(moves, 1000)!;
    expect(p.x === 0.2 || p.x === 0.8).toBe(true);
  });

  it("out-of-order input is sorted defensively, and the input array is not mutated", () => {
    const moves = [kf(2000, 0.9, 0.9, 0.5, "linear"), kf(0, 0.0, 0.0, 0.0, "linear")];
    const snapshot = [...moves];
    expect(camMoveAt(moves, 0)).toEqual({ x: 0.0, y: 0.0, size: 0.0 });
    expect(camMoveAt(moves, 2000)).toEqual({ x: 0.9, y: 0.9, size: 0.5 });
    const mid = camMoveAt(moves, 1000)!;
    expect(mid.x).toBeCloseTo(0.45, 6);
    expect(moves).toEqual(snapshot); // camMoveAt must sort a copy, not the caller's array
  });
});
