import { describe, expect, it } from "vitest";
import type { CameraMove } from "../../lib/edit";
import { ease } from "../timeline/layoutTrack";
import { camMoveAt, radiusScaleForResize, rectFromCenter } from "./cameraMoves";

// Mirrors src-tauri/src/export/camera/moves_tests.rs - the Rust authority's cases,
// re-asserted in TS so the two never silently diverge. The Task 27 SPAN semantics (what the
// keyframes own, and the two blends at its edges) live in camMoveSpan.test.ts, mirroring the
// Rust split into moves_tests.rs / moves_span_tests.rs.
const kf = (t_ms: number, x: number, y: number, size: number, easing: string): CameraMove =>
  ({ id: "k", t_ms, x, y, size, easing });

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

  // Mirrors moves_tests.rs's mid_span_interpolation_is_unchanged_by_the_span_rewrite - the
  // same keyframes (2000/4000) sampled at the same t (3000), so any drift in the in-span math
  // fails on BOTH sides at once.
  it("mid-span interpolation is unchanged by the span rewrite, and ignores the live pose", () => {
    const moves = [kf(2000, 0.10, 0.80, 0.12, "smooth"), kf(4000, 0.70, 0.20, 0.34, "smooth")];
    const f = ease("smooth", 0.5); // === 0.5
    const want = { x: 0.10 + 0.60 * f, y: 0.80 - 0.60 * f, size: 0.12 + 0.22 * f };
    expect(want.x).toBeCloseTo(0.40, 6); // pinned pre-change value
    for (const got of [camMoveAt(moves, 3000)!, camMoveAt(moves, 3000, { x: 0.95, y: 0.05, size: 0.9 })!]) {
      expect(got.x).toBeCloseTo(want.x, 4);
      expect(got.y).toBeCloseTo(want.y, 4);
      expect(got.size).toBeCloseTo(want.size, 4);
    }
  });
});

// Task 9 Part C: mirrors export::scene::mod_tests::override_camera_* - the preview must scale
// the spliced radius fraction by the same height ratio the Rust `override_camera` uses, so a
// circle stays a true circle after a keyframe resizes the panel (not just in the export).
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
    // PreviewLayout.cam[4] is a WIDTH-relative fraction (radius_px / ow - see previewCanvas.ts's
    // `wr = fr * w`), while cam[2]/cam[3] (w/h) are relative to ow/oh respectively - so comparing
    // fractions directly (as in Rust, where both are already px) is invalid whenever ow != oh.
    // Converting each back to pixels with its own axis is what actually proves parity.
    const ow = 1280, oh = 720;
    const oldHFrac = 0.2, oldHPx = oldHFrac * oh; // static panel: square in px, height 0.2 of oh
    const staticRadiusFrac = (oldHPx / 2) / ow;    // radius_px (== oldHPx/2 for a circle) / ow
    const cp = { x: 0.5, y: 0.5, size: 0.05 };     // shrinks to a 0.05-tall (of oh) panel
    const [, , , newHFrac] = rectFromCenter(cp, ow, oh, 1);
    const newHPx = newHFrac * oh;
    const scaledRadiusFrac = staticRadiusFrac * radiusScaleForResize(oldHFrac, newHFrac);
    const scaledRadiusPx = scaledRadiusFrac * ow;
    expect(scaledRadiusPx).toBeCloseTo(newHPx / 2, 5); // still a true circle: radius == h/2 in px
  });

  // Mirrors export::scene::cam_tests::rect_from_center_keeps_a_wide_panel_wide - a pose
  // carries height only, so a Wide (16:9) PiP must get its width back from the aspect
  // instead of being squared by the first keyframe.
  it("keeps a wide panel wide (aspect 16/9), still centred on the pose", () => {
    const ow = 1920, oh = 1080, a = 16 / 9;
    const [x, y, w, h] = rectFromCenter({ x: 0.5, y: 0.5, size: 0.25 }, ow, oh, a);
    expect(h * oh).toBeCloseTo(0.25 * oh, 6);
    expect((w * ow) / (h * oh)).toBeCloseTo(a, 6);
    expect(x + w / 2).toBeCloseTo(0.5, 6);
    expect(y + h / 2).toBeCloseTo(0.5, 6);
  });

  it("aspect 1 reproduces the old pixel-square behavior exactly", () => {
    const ow = 1920, oh = 1080;
    const [, , w, h] = rectFromCenter({ x: 0.3, y: 0.7, size: 0.2 }, ow, oh, 1);
    expect(w * ow).toBeCloseTo(h * oh, 6); // square in PIXELS even though ow != oh
  });
});
