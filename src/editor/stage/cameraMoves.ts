import type { CameraMove } from "../../lib/edit";
import { ease } from "../timeline/layoutTrack";

export interface CamPose { x: number; y: number; size: number }

/** TS mirror of `CameraMoveTrack::sample` (export/camera/moves.rs) - the Rust export path is
 *  the source of truth, this drives the live preview and must match it frame-for-frame.
 *  `null` for an empty track (caller keeps its static pose). At/before the first keyframe,
 *  eases FROM `staticPose` (the caller's un-overridden static PiP pose) INTO the first keyframe
 *  over `[0, first.t_ms]` using the first keyframe's own easing - an implicit t=0 keyframe at
 *  the static pose; `staticPose` omitted/null (or `first.t_ms === 0`) holds the first keyframe's
 *  pose flat, same as before this existed. Holds the last keyframe's pose outside the track's
 *  span; between two keyframes `a`/`b`, eases INTO `b` using `b`'s own easing over
 *  `[a.t_ms, b.t_ms]`. */
export function camMoveAt(moves: CameraMove[], t: number, staticPose?: CamPose | null): CamPose | null {
  const ks = [...moves].sort((a, b) => a.t_ms - b.t_ms);
  if (ks.length === 0) return null;
  const first = ks[0];
  if (t <= first.t_ms) {
    if (staticPose && first.t_ms > 0) {
      const f = ease(first.easing, t / first.t_ms);
      return { x: staticPose.x + (first.x - staticPose.x) * f, y: staticPose.y + (first.y - staticPose.y) * f,
        size: staticPose.size + (first.size - staticPose.size) * f };
    }
    return pose(first);
  }
  const last = ks[ks.length - 1];
  if (t >= last.t_ms) return pose(last);

  const bi = ks.findIndex((k) => k.t_ms > t);
  const a = ks[bi - 1], b = ks[bi];
  if (b.t_ms === a.t_ms) return pose(b); // coincident keyframes: no divide-by-zero
  const f = ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms));
  return { x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, size: a.size + (b.size - a.size) * f };
}

function pose(k: CameraMove): CamPose { return { x: k.x, y: k.y, size: k.size }; }

/** TS mirror of `rect_from_center` (export/scene/mod.rs) - converts a sampled `CamPose` into a
 *  fraction-of-output `[x, y, w, h]` rect (top-left form), matching the export byte-for-byte.
 *  `ow`/`oh` are the output frame's pixel dims (the preview's fixed 1280x720 backing store,
 *  same basis `PreviewLayout` fractions use) - the square is computed in PIXEL space (`h = size*oh`
 *  reused for both sides) before dividing back to fractions, so it stays a true pixel square even
 *  though `ow != oh` would otherwise skew it. */
export function rectFromCenter(p: CamPose, ow: number, oh: number): [number, number, number, number] {
  const h = p.size * oh;
  return [(p.x * ow - h / 2) / ow, (p.y * oh - h / 2) / oh, h / ow, h / oh];
}

/** TS mirror of `override_camera`'s radius scaling (export/scene/mod.rs, Task 9 Part C) - the
 *  factor to multiply the STATIC (unoverridden) radius fraction by so a circle stays a true
 *  circle after a camera_moves keyframe grows/shrinks the panel. `oldH`/`newH` are the panel
 *  height fraction before/after the override (`PreviewLayout.cam[3]` before vs. `rectFromCenter`'s
 *  `[3]` after) - same units, so the ratio is dimension-independent. Guards `oldH` against 0 the
 *  same way the Rust does (`old_h.max(0.001)`). */
export function radiusScaleForResize(oldH: number, newH: number): number {
  return newH / Math.max(oldH, 0.001);
}

/** TS mirror of `override_camera` (export/scene/mod.rs) end-to-end: replaces `baseCam`'s rect with
 *  the sampled pose and scales BOTH radius and ring width by the same height ratio, so a circle
 *  (and its ring) stay proportional after a camera_moves keyframe resizes the panel; ring color
 *  (and alpha, left to the caller) is untouched - matches `Panel { rect, radius: r*m, ring_px:
 *  ring_px*m, ..panel }`. `baseCam` is `PreviewLayout.cam` (non-null, [x,y,w,h,r,ringPx,r,g,b]). */
export function overrideCamPanel(
  baseCam: [number, number, number, number, number, number, number, number, number],
  p: CamPose, ow: number, oh: number,
): [number, number, number, number, number, number, number, number, number] {
  const newRect = rectFromCenter(p, ow, oh);
  const scale = radiusScaleForResize(baseCam[3], newRect[3]);
  return [...newRect, baseCam[4] * scale, baseCam[5] * scale, baseCam[6], baseCam[7], baseCam[8]];
}
