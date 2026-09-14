import type { CameraMove, CamMoveShape, EditDoc, EditOp } from "../../lib/edit";

/** Frames within this many ms of the playhead are treated as "the same keyframe" - editing
 *  the size slider or dragging the PiP updates that keyframe in place instead of stacking a
 *  near-duplicate one a frame or two away. */
export const CAM_KF_SNAP_MS = 60;

/** The fields a keyframe commit may set. `shape`/`roundness` are the keyframe's own shape
 *  (`"layout"` = inherit the layout's); a brand-new keyframe leaves them to the backend's
 *  defaults when the patch does not carry them. */
export interface CamKfPatch { x?: number; y?: number; size?: number; shape?: CamMoveShape; roundness?: number }

/** Finds the `camera_moves` entry considered "at" `timeMs` (within `CAM_KF_SNAP_MS`), or
 *  `null` if none is that close. Shared by the drag-commit (Stage) and the Webcam-size
 *  slider (CameraPanel) so both agree on when to update vs. create a keyframe. */
export function camKeyframeAt(moves: CameraMove[], timeMs: number): CameraMove | null {
  const t = Math.round(timeMs);
  let best: CameraMove | null = null;
  let bestDist = Infinity;
  for (const m of moves) {
    const d = Math.abs(m.t_ms - t);
    if (d <= CAM_KF_SNAP_MS && d < bestDist) { best = m; bestDist = d; }
  }
  return best;
}

/** Commits a `camera_moves` keyframe at the playhead: updates the existing one within the
 *  snap window (patching only the given fields), else adds a new one at `Math.round(timeMs)`.
 *  `fallback` supplies the pose a brand-new keyframe needs but the caller didn't set
 *  (e.g. the drag commits x/y and keeps the current size; the size slider commits size and
 *  keeps the current x/y). The backend folds an add at an instant that already holds a keyframe
 *  into an in-place update, so two commits racing a doc round-trip cannot stack duplicates. */
export async function commitCamKeyframe(
  moves: CameraMove[],
  timeMs: number,
  patch: CamKfPatch,
  fallback: { x: number; y: number; size: number },
  onApply: (op: EditOp) => Promise<EditDoc | null>,
): Promise<EditDoc | null> {
  const existing = camKeyframeAt(moves, timeMs);
  if (existing) {
    return onApply({ op: "update_camera_move", id: existing.id, ...patch });
  }
  return onApply({
    op: "add_camera_move",
    t_ms: Math.round(timeMs),
    x: patch.x ?? fallback.x,
    y: patch.y ?? fallback.y,
    size: patch.size ?? fallback.size,
    ...(patch.shape !== undefined ? { shape: patch.shape } : {}),
    ...(patch.roundness !== undefined ? { roundness: patch.roundness } : {}),
  });
}
