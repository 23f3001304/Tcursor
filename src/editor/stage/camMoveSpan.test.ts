import { describe, expect, it } from "vitest";
import type { CameraMove } from "../../lib/edit";
import { camKfRange, camMoveAt, KF_BLEND_MS } from "./cameraMoves";

// Task 27 span semantics, mirroring src-tauri/src/export/camera/moves_span_tests.rs with the
// SAME numbers: keyframes own only [first - KF_BLEND_MS, last + KF_BLEND_MS], easing to and from
// the live layout-resolved pose at each edge. Outside it, `camMoveAt` is null and the layout
// segments own the webcam panel.
const kf = (t_ms: number, x: number, y: number, size: number, easing: string): CameraMove =>
  ({ id: `k${t_ms}`, t_ms, x, y, size, easing, shape: "layout", roundness: 0.12 });

const LIVE = { x: 0.85, y: 0.15, size: 0.18 };
/** Two keyframes at 2000/4000 - the span is [1650, 4350]. */
const twoKf = () => [kf(2000, 0.20, 0.70, 0.10, "linear"), kf(4000, 0.60, 0.30, 0.40, "linear")];

describe("camMoveAt span semantics", () => {
  it("KF_BLEND_MS matches the Rust constant", () => {
    expect(KF_BLEND_MS).toBe(350);
  });

  it("camKfRange reports the RAW keyframe range (unpadded), or null when empty", () => {
    expect(camKfRange([])).toBeNull();
    expect(camKfRange(twoKf())).toEqual([2000, 4000]);
    expect(camKfRange([kf(4000, 0, 0, 0, "linear"), kf(2000, 0, 0, 0, "linear")])).toEqual([2000, 4000]);
    // Explicitly NOT the ownership window - that is this padded by KF_BLEND_MS, which is what
    // camMoveAt tests against and what Rust's CameraMoveTrack::span returns.
    expect(camMoveAt(twoKf(), 2000 - KF_BLEND_MS, LIVE)).not.toBeNull();
  });

  it("case 1: outside the span the layout owns the panel", () => {
    const moves = twoKf();
    expect(camMoveAt(moves, 1000, LIVE)).toBeNull();
    expect(camMoveAt(moves, 5000, LIVE)).toBeNull();
    expect(camMoveAt(moves, 1649, LIVE)).toBeNull();
    expect(camMoveAt(moves, 1650, LIVE)).not.toBeNull(); // the edges are INSIDE
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
    expect(atFirst.x).toBeCloseTo(0.20, 4);
    expect(atFirst.y).toBeCloseTo(0.70, 4);
    expect(atFirst.size).toBeCloseTo(0.10, 4);

    const mid = camMoveAt(moves, 2000 - KF_BLEND_MS / 2, LIVE)!;
    expect(mid.x).toBeLessThan(LIVE.x); expect(mid.x).toBeGreaterThan(0.20);
    expect(mid.y).toBeGreaterThan(LIVE.y); expect(mid.y).toBeLessThan(0.70);
    expect(mid.size).toBeLessThan(LIVE.size); expect(mid.size).toBeGreaterThan(0.10);
  });

  it("case 4: the exit blend tracks a live pose that is still moving", () => {
    // A layout transition running THROUGH the exit window: live(t) sweeps from (0.2,0.2,0.1) at
    // t=last to (0.8,0.6,0.3) at t=last+BLEND. Re-evaluating live per frame means the blend lands
    // on live(last+BLEND) - the moving target - not the stale live(last) it started from.
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
    expect(Math.abs(got.x - liveAt(4000).x)).toBeGreaterThan(0.5); // not the stale start pose
    expect(camMoveAt(moves, 4000, liveAt(4000))!.x).toBeCloseTo(0.60, 4); // exit hasn't begun yet
  });

  it("case 5: a single keyframe is a bump, not a whole-clip override", () => {
    const moves = [kf(3000, 0.25, 0.75, 0.30, "linear")];
    expect(camMoveAt(moves, 2649, LIVE)).toBeNull();
    expect(camMoveAt(moves, 3351, LIVE)).toBeNull();
    expect(camMoveAt(moves, 3000, LIVE)).toEqual({ x: 0.25, y: 0.75, size: 0.30 });
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
    expect(camMoveAt(moves, 1700)).toEqual({ x: 0.20, y: 0.70, size: 0.10 });
    expect(camMoveAt(moves, 4300)).toEqual({ x: 0.60, y: 0.30, size: 0.40 });
  });
});
