import { describe, expect, it } from "vitest";
import type { CameraMove } from "../../../shared/edit";
import { ease } from "../../timeline/model/layoutTrack";
import { camMoveAt, cameraMovesKey, radiusScaleForResize, rectFromCenter } from "./cameraMoves";

const kf = (t_ms: number, x: number, y: number, size: number, easing: string, id = "k"): CameraMove => ({
  id,
  t_ms,
  x,
  y,
  size,
  easing,
  shape: "layout",
  roundness: 0.12,
});

describe("camMoveAt", () => {
  it("empty track samples to null", () => {
    expect(camMoveAt([], 0)).toBeNull();
    expect(camMoveAt([], 5000)).toBeNull();
  });

  it("holds the last pose exactly AT the last keyframe", () => {
    const moves = [kf(1000, 0.1, 0.1, 0.1, "linear"), kf(2000, 0.9, 0.9, 0.5, "linear")];
    expect(camMoveAt(moves, 2000)).toEqual({ x: 0.9, y: 0.9, size: 0.5 });
  });

  it("midpoint lerp with linear easing is the exact mean", () => {
    const moves = [kf(0, 0.0, 0.0, 0.0, "linear"), kf(1000, 1.0, 1.0, 1.0, "linear")];
    const p = camMoveAt(moves, 500)!;
    expect(p.x).toBeCloseTo(0.5, 6);
    expect(p.y).toBeCloseTo(0.5, 6);
    expect(p.size).toBeCloseTo(0.5, 6);
  });

  it("midpoint with smooth easing diverges from the linear mean", () => {
    const moves = [kf(0, 0.0, 0.2, 0.1, "smooth"), kf(1000, 1.0, 0.2, 0.9, "smooth")];
    const f = ease("smooth", 0.25);
    expect(Math.abs(f - 0.25)).toBeGreaterThan(1e-6);
    const p = camMoveAt(moves, 250)!;
    expect(p.x).toBeCloseTo(f, 6);
    expect(p.size).toBeCloseTo(0.1 + 0.8 * f, 6);
  });

  it("coincident keyframe times snap to b without dividing by zero", () => {
    const moves = [
      kf(1000, 0.2, 0.2, 0.2, "linear"),
      kf(1000, 0.8, 0.8, 0.8, "linear"),
      kf(2000, 0.0, 0.0, 0.0, "linear"),
    ];
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
    expect(moves).toEqual(snapshot);
  });

  it("mid-span interpolation is unchanged by the span rewrite, and ignores the live pose", () => {
    const moves = [kf(2000, 0.1, 0.8, 0.12, "smooth"), kf(4000, 0.7, 0.2, 0.34, "smooth")];
    const f = ease("smooth", 0.5);
    const want = { x: 0.1 + 0.6 * f, y: 0.8 - 0.6 * f, size: 0.12 + 0.22 * f };
    expect(want.x).toBeCloseTo(0.4, 6);
    for (const got of [camMoveAt(moves, 3000)!, camMoveAt(moves, 3000, { x: 0.95, y: 0.05, size: 0.9 })!]) {
      expect(got.x).toBeCloseTo(want.x, 4);
      expect(got.y).toBeCloseTo(want.y, 4);
      expect(got.size).toBeCloseTo(want.size, 4);
    }
  });
});

describe("radiusScaleForResize", () => {
  it("shrinking the panel shrinks the scale proportionally (half height -> half radius)", () => {
    expect(radiusScaleForResize(0.2, 0.1)).toBeCloseTo(0.5, 6);
  });

  it("growing the panel grows the scale proportionally (4x height -> 4x radius)", () => {
    expect(radiusScaleForResize(0.1, 0.4)).toBeCloseTo(4.0, 6);
  });

  it("no size change is a no-op scale of 1", () => {
    expect(radiusScaleForResize(0.25, 0.25)).toBe(1);
  });

  it("guards a zero/near-zero old height against dividing by zero (matches Rust's old_h.max(0.001))", () => {
    expect(Number.isFinite(radiusScaleForResize(0, 0.1))).toBe(true);
    expect(radiusScaleForResize(0, 0.1)).toBeCloseTo(0.1 / 0.001, 6);
  });

  it("end-to-end: a circle's radius stays min(w,h)/2 IN PIXELS after resize, mirroring the export", () => {
    const ow = 1280,
      oh = 720;
    const oldHFrac = 0.2,
      oldHPx = oldHFrac * oh;
    const staticRadiusFrac = oldHPx / 2 / ow;
    const cp = { x: 0.5, y: 0.5, size: 0.05 };
    const [, , , newHFrac] = rectFromCenter(cp, ow, oh, 1);
    const newHPx = newHFrac * oh;
    const scaledRadiusFrac = staticRadiusFrac * radiusScaleForResize(oldHFrac, newHFrac);
    const scaledRadiusPx = scaledRadiusFrac * ow;
    expect(scaledRadiusPx).toBeCloseTo(newHPx / 2, 5);
  });

  it("keeps a wide panel wide (aspect 16/9), still centred on the pose", () => {
    const ow = 1920,
      oh = 1080,
      a = 16 / 9;
    const [x, y, w, h] = rectFromCenter({ x: 0.5, y: 0.5, size: 0.25 }, ow, oh, a);
    expect(h * oh).toBeCloseTo(0.25 * oh, 6);
    expect((w * ow) / (h * oh)).toBeCloseTo(a, 6);
    expect(x + w / 2).toBeCloseTo(0.5, 6);
    expect(y + h / 2).toBeCloseTo(0.5, 6);
  });

  it("aspect 1 reproduces the old pixel-square behavior exactly", () => {
    const ow = 1920,
      oh = 1080;
    const [, , w, h] = rectFromCenter({ x: 0.3, y: 0.7, size: 0.2 }, ow, oh, 1);
    expect(w * ow).toBeCloseTo(h * oh, 6);
  });
});

describe("cameraMovesKey", () => {
  it("is stable for content-identical arrays even with a different array/object reference", () => {
    const a = [kf(1000, 0.1, 0.2, 0.3, "smooth", "k1"), kf(2000, 0.4, 0.5, 0.6, "linear", "k2")];
    const b = [kf(1000, 0.1, 0.2, 0.3, "smooth", "k1"), kf(2000, 0.4, 0.5, 0.6, "linear", "k2")];
    expect(a).not.toBe(b);
    expect(cameraMovesKey(a)).toBe(cameraMovesKey(b));
  });

  it("is empty-but-deterministic for an empty track", () => {
    expect(cameraMovesKey([])).toBe(cameraMovesKey([]));
  });

  it("changes when a keyframe is added", () => {
    const before = [kf(1000, 0.1, 0.1, 0.1, "linear")];
    const after = [...before, kf(2000, 0.9, 0.9, 0.9, "linear", "k2")];
    expect(cameraMovesKey(before)).not.toBe(cameraMovesKey(after));
  });

  it("changes when a keyframe is removed", () => {
    const before = [kf(1000, 0.1, 0.1, 0.1, "linear", "k1"), kf(2000, 0.9, 0.9, 0.9, "linear", "k2")];
    const after = before.slice(0, 1);
    expect(cameraMovesKey(before)).not.toBe(cameraMovesKey(after));
  });

  it("changes when any single field of one keyframe changes (t_ms, x, y, size, easing, id)", () => {
    const base = kf(1000, 0.1, 0.2, 0.3, "smooth", "k1");
    const baseKey = cameraMovesKey([base]);
    expect(cameraMovesKey([{ ...base, t_ms: 1001 }])).not.toBe(baseKey);
    expect(cameraMovesKey([{ ...base, x: 0.11 }])).not.toBe(baseKey);
    expect(cameraMovesKey([{ ...base, y: 0.21 }])).not.toBe(baseKey);
    expect(cameraMovesKey([{ ...base, size: 0.31 }])).not.toBe(baseKey);
    expect(cameraMovesKey([{ ...base, easing: "linear" }])).not.toBe(baseKey);
    expect(cameraMovesKey([{ ...base, id: "k2" }])).not.toBe(baseKey);
  });

  it("is sensitive to array order (a same-set reorder still counts as a change)", () => {
    const a = [kf(1000, 0.1, 0.1, 0.1, "linear", "k1"), kf(2000, 0.9, 0.9, 0.9, "linear", "k2")];
    const b = [a[1], a[0]];
    expect(cameraMovesKey(a)).not.toBe(cameraMovesKey(b));
  });
});
