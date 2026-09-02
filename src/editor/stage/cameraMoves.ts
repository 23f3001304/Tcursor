import type { CameraMove } from "../../lib/edit";
import { ease } from "../timeline/layoutTrack";

export interface CamPose { x: number; y: number; size: number }

/** Handoff length (ms) on EACH side of the keyframe span, between the live layout-resolved pose
 *  and the track. Mirrors the Rust `KF_BLEND_MS` (export/camera/moves.rs) - a module constant,
 *  deliberately not a setting; the two MUST stay equal. */
export const KF_BLEND_MS = 350;

/** The RAW `[first, last]` keyframe times of a track (unsorted input is fine), or `null` when
 *  empty. NOT the ownership window: that is this padded by `KF_BLEND_MS` on each side (clamped at
 *  0), which is what Rust's `CameraMoveTrack::span` returns and what `camMoveAt` tests against.
 *  Callers that want the window pad it themselves - kept unpadded because both consumers need the
 *  raw ends (one clamps a time into them, one pads them with its own drag-adjusted values). */
export function camKfRange(moves: CameraMove[]): [number, number] | null {
  if (moves.length === 0) return null;
  let first = Infinity, last = -Infinity;
  for (const m of moves) { if (m.t_ms < first) first = m.t_ms; if (m.t_ms > last) last = m.t_ms; }
  return [first, last];
}

/** TS mirror of `CameraMoveTrack::sample` (export/camera/moves.rs) - the Rust export path is
 *  the source of truth, this drives the live preview and must match it frame-for-frame.
 *  `null` means the keyframes do NOT own this frame and the caller keeps its layout-resolved
 *  panel. `live` is that layout-resolved pose for THIS frame, re-read every call.
 *
 *  Five cases: empty track or outside `[first - KF_BLEND_MS, last + KF_BLEND_MS]` -> `null`;
 *  `[first - BLEND, first)` -> ease FROM `live` INTO the first keyframe with its own easing;
 *  `[first, last]` -> keyframe interpolation, easing INTO `b` with `b`'s own easing (unchanged);
 *  `(last, last + BLEND]` -> ease FROM the last keyframe back to `live`, which tracks a moving
 *  target because it is re-evaluated per frame. A single keyframe is therefore a bump: ease in,
 *  hit its pose for that instant, ease back out. `live` omitted/null skips both blends and snaps
 *  to the nearest end keyframe; the span rule itself never depends on it. */
export function camMoveAt(moves: CameraMove[], t: number, live?: CamPose | null): CamPose | null {
  const ks = [...moves].sort((a, b) => a.t_ms - b.t_ms);
  if (ks.length === 0) return null;
  const first = ks[0], last = ks[ks.length - 1];
  const entry = Math.max(0, first.t_ms - KF_BLEND_MS);
  if (t < entry || t > last.t_ms + KF_BLEND_MS) return null;
  if (t < first.t_ms) {
    const win = first.t_ms - entry; // === KF_BLEND_MS unless clamped at t=0
    if (!live || win <= 0) return pose(first);
    return mix(live, pose(first), ease(first.easing, (t - entry) / win));
  }
  if (t > last.t_ms) {
    if (!live) return pose(last);
    return mix(pose(last), live, ease(last.easing, (t - last.t_ms) / KF_BLEND_MS));
  }
  if (t >= last.t_ms) return pose(last); // also the single-keyframe instant

  const bi = ks.findIndex((k) => k.t_ms > t);
  const a = ks[bi - 1], b = ks[bi];
  if (b.t_ms === a.t_ms) return pose(b); // coincident keyframes: no divide-by-zero
  return mix(pose(a), pose(b), ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms)));
}

function pose(k: CameraMove): CamPose { return { x: k.x, y: k.y, size: k.size }; }
/** Component-wise lerp of a whole pose; the rect is derived from the result once, by
 *  `rectFromCenter`, so the aspect handling applies to a blended pose too. Mirrors Rust `mix`. */
function mix(a: CamPose, b: CamPose, f: number): CamPose {
  return { x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, size: a.size + (b.size - a.size) * f };
}

/** TS mirror of `rect_from_center` (export/scene/mod.rs) - converts a sampled `CamPose` into a
 *  fraction-of-output `[x, y, w, h]` rect (top-left form), matching the export byte-for-byte.
 *  `ow`/`oh` are the output frame's pixel dims (the preview's canvas backing store, sized from
 *  `PreviewLayout.canvas` to match the chosen aspect - same basis `PreviewLayout` fractions use) -
 *  the rect is computed in PIXEL space (`h = size*oh`, `w = h*aspect`) before dividing back to
 *  fractions, so it keeps its true pixel shape even though `ow != oh` would otherwise skew it.
 *  `aspect` is the panel's own w/h in pixels (1 = square, 16/9 = a Wide PiP): a pose carries
 *  height only, so without it one keyframe would square a Wide panel for the whole clip. */
export function rectFromCenter(p: CamPose, ow: number, oh: number, aspect: number): [number, number, number, number] {
  const h = p.size * oh;
  const w = h * Math.max(aspect, 0.01);
  return [(p.x * ow - w / 2) / ow, (p.y * oh - h / 2) / oh, w / ow, h / oh];
}

/** The static PiP panel's pixel aspect (w/h) from a `PreviewLayout.cam` tuple, whose `w`/`h`
 *  are fractions of DIFFERENT axes (`ow`/`oh`) and so must be converted back to pixels first.
 *  This is the `aspect` `rectFromCenter` needs, and mirrors the renderer reading it off
 *  `scene.camera.rect` (export/render/mod.rs) rather than re-deriving it from settings. */
export function camAspect(cam: [number, number, number, number, ...number[]], ow: number, oh: number): number {
  return (cam[2] * ow) / Math.max(cam[3] * oh, 0.001);
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
 *  ring_px*m, ..panel }`. `baseCam` is `PreviewLayout.cam` (non-null, [x,y,w,h,r,ringPx,r,g,b]).
 *  The keyframed rect keeps the STATIC panel's pixel aspect (`baseCam` w/h converted to px),
 *  mirroring how the renderer reads `scene.camera.rect` before applying the override. */
export function overrideCamPanel(
  baseCam: [number, number, number, number, number, number, number, number, number],
  p: CamPose, ow: number, oh: number,
): [number, number, number, number, number, number, number, number, number] {
  const newRect = rectFromCenter(p, ow, oh, camAspect(baseCam, ow, oh));
  const scale = radiusScaleForResize(baseCam[3], newRect[3]);
  return [...newRect, baseCam[4] * scale, baseCam[5] * scale, baseCam[6], baseCam[7], baseCam[8]];
}

/** A cheap CONTENT signature for `moves` - id/t_ms/x/y/size/easing per entry, joined in array
 *  order. Pure and unit-tested (`cameraMoves.test.ts`) so the exact "what counts as a content
 *  change" contract is verifiable without mounting anything (review round 2, Important).
 *
 *  *Why this exists:* `applyEditOp` round-trips the WHOLE `EditDoc` through IPC, so `setDoc`
 *  hands back a brand-new `camera_moves` ARRAY REFERENCE on every edit routed through it - adding
 *  a zoom, deleting a region, trimming, an AI-director step - not just camera-move ones. A
 *  `useEffect` keyed on `[cameraMoves]` (reference) fires on ALL of those, not just the ones that
 *  actually changed camera-move content; comparing this key instead of the array reference lets a
 *  caller (`CamDragHandle.tsx`) tell "an unrelated edit refreshed the doc" apart from "a
 *  camera-move keyframe was actually added/updated/removed". */
export function cameraMovesKey(moves: CameraMove[]): string {
  return moves.map((m) => `${m.id}:${m.t_ms}:${m.x}:${m.y}:${m.size}:${m.easing}`).join("|");
}
