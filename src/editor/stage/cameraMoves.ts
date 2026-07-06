import type { CameraMove } from "../../lib/edit";
import { ease } from "../timeline/layoutTrack";

export interface CamPose { x: number; y: number; size: number }

/** TS mirror of `CameraMoveTrack::sample` (export/camera/moves.rs) - the Rust export path is
 *  the source of truth, this drives the live preview and must match it frame-for-frame.
 *  `null` for an empty track (caller keeps its static pose); holds the first/last keyframe's
 *  pose outside the track's span; between two keyframes `a`/`b`, eases INTO `b` using `b`'s
 *  own easing over `[a.t_ms, b.t_ms]`. */
export function camMoveAt(moves: CameraMove[], t: number): CamPose | null {
  const ks = [...moves].sort((a, b) => a.t_ms - b.t_ms);
  if (ks.length === 0) return null;
  if (t <= ks[0].t_ms) return pose(ks[0]);
  const last = ks[ks.length - 1];
  if (t >= last.t_ms) return pose(last);

  const bi = ks.findIndex((k) => k.t_ms > t);
  const a = ks[bi - 1], b = ks[bi];
  if (b.t_ms === a.t_ms) return pose(b); // coincident keyframes: no divide-by-zero
  const f = ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms));
  return { x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, size: a.size + (b.size - a.size) * f };
}

function pose(k: CameraMove): CamPose { return { x: k.x, y: k.y, size: k.size }; }
