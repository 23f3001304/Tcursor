// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import type { CameraMove } from "../../../shared/edit";
import { camKfRange, camMoveAt, KF_BLEND_MS } from "./cameraMoves";

const kf = (t_ms: number, x: number, y: number, size: number, easing: string): CameraMove => ({
  id: `k${t_ms}`,
  t_ms,
  x,
  y,
  size,
  easing,
  shape: "layout",
  roundness: 0.12,
});

const LIVE = { x: 0.85, y: 0.15, size: 0.18 };

const twoKf = () => [kf(2000, 0.2, 0.7, 0.1, "linear"), kf(4000, 0.6, 0.3, 0.4, "linear")];

describe("camMoveAt span semantics", () => {
  it("KF_BLEND_MS matches the Rust constant", () => {
    expect(KF_BLEND_MS).toBe(350);
  });

  it("camKfRange reports the RAW keyframe range (unpadded), or null when empty", () => {
    expect(camKfRange([])).toBeNull();
    expect(camKfRange(twoKf())).toEqual([2000, 4000]);
    expect(camKfRange([kf(4000, 0, 0, 0, "linear"), kf(2000, 0, 0, 0, "linear")])).toEqual([2000, 4000]);
    expect(camMoveAt(twoKf(), 2000 - KF_BLEND_MS, LIVE)).not.toBeNull();
  });

  it("case 1: outside the span the layout owns the panel", () => {
    const moves = twoKf();
    expect(camMoveAt(moves, 1000, LIVE)).toBeNull();
    expect(camMoveAt(moves, 5000, LIVE)).toBeNull();
    expect(camMoveAt(moves, 1649, LIVE)).toBeNull();
    expect(camMoveAt(moves, 1650, LIVE)).not.toBeNull();
    expect(camMoveAt(moves, 4350, LIVE)).not.toBeNull();
    expect(camMoveAt(moves, 4351, LIVE)).toBeNull();
  });

  it("case 2: the entry blend is continuous at both ends", () => {
    const moves = twoKf();
    const atStart = camMoveAt(moves, 2000 - KF_BLEND_MS, LIVE)!;
    expect(atStart.x).toBeCloseTo(LIVE.x, 4);
    expect(atStart.y).toBeCloseTo(LIVE.y, 4);
    expect(atStart.size).toBeCloseTo(LIVE.size, 4);

    const atFirst = camMoveAt(moves, 2000, LIVE)!;
    expect(atFirst.x).toBeCloseTo(0.2, 4);
    expect(atFirst.y).toBeCloseTo(0.7, 4);
    expect(atFirst.size).toBeCloseTo(0.1, 4);

    const mid = camMoveAt(moves, 2000 - KF_BLEND_MS / 2, LIVE)!;
    expect(mid.x).toBeLessThan(LIVE.x);
    expect(mid.x).toBeGreaterThan(0.2);
    expect(mid.y).toBeGreaterThan(LIVE.y);
    expect(mid.y).toBeLessThan(0.7);
    expect(mid.size).toBeLessThan(LIVE.size);
    expect(mid.size).toBeGreaterThan(0.1);
  });

  it("case 4: the exit blend tracks a live pose that is still moving", () => {
    const moves = twoKf();
    const liveAt = (t: number) => {
      const u = (t - 4000) / KF_BLEND_MS;
      return { x: 0.2 + 0.6 * u, y: 0.2 + 0.4 * u, size: 0.1 + 0.2 * u };
    };
    const endT = 4000 + KF_BLEND_MS;
    const got = camMoveAt(moves, endT, liveAt(endT))!;
    expect(got.x).toBeCloseTo(0.8, 4);
    expect(got.y).toBeCloseTo(0.6, 4);
    expect(got.size).toBeCloseTo(0.3, 4);
    expect(Math.abs(got.x - liveAt(4000).x)).toBeGreaterThan(0.5);
    expect(camMoveAt(moves, 4000, liveAt(4000))!.x).toBeCloseTo(0.6, 4);
  });

  it("case 5: a single keyframe is a bump, not a whole-clip override", () => {
    const moves = [kf(3000, 0.25, 0.75, 0.3, "linear")];
    expect(camMoveAt(moves, 2649, LIVE)).toBeNull();
    expect(camMoveAt(moves, 3351, LIVE)).toBeNull();
    expect(camMoveAt(moves, 3000, LIVE)).toEqual({ x: 0.25, y: 0.75, size: 0.3 });
    for (const t of [2650, 3350]) {
      const p = camMoveAt(moves, t, LIVE)!;
      expect(p.x).toBeCloseTo(LIVE.x, 4);
      expect(p.size).toBeCloseTo(LIVE.size, 4);
    }
  });

  it("without a live pose the blends snap to the nearest end keyframe (span rule unchanged)", () => {
    const moves = twoKf();
    expect(camMoveAt(moves, 1000)).toBeNull();
    expect(camMoveAt(moves, 5000)).toBeNull();
    expect(camMoveAt(moves, 1700)).toEqual({ x: 0.2, y: 0.7, size: 0.1 });
    expect(camMoveAt(moves, 4300)).toEqual({ x: 0.6, y: 0.3, size: 0.4 });
  });
});
