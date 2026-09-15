import type { CameraMove, CamMoveShape, EditDoc, EditOp } from "../../../shared/edit";

export const CAM_KF_SNAP_MS = 60;

export interface CamKfPatch {
  x?: number;
  y?: number;
  size?: number;
  shape?: CamMoveShape;
  roundness?: number;
}

export function camKeyframeAt(moves: CameraMove[], timeMs: number): CameraMove | null {
  const t = Math.round(timeMs);
  let best: CameraMove | null = null;
  let bestDist = Infinity;
  for (const m of moves) {
    const d = Math.abs(m.t_ms - t);
    if (d <= CAM_KF_SNAP_MS && d < bestDist) {
      best = m;
      bestDist = d;
    }
  }
  return best;
}

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
