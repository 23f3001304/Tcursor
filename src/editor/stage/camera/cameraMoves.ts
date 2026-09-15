import type { CameraMove } from "../../../shared/edit";
import { ease } from "../../timeline/model/layoutTrack";

export interface CamPose {
  x: number;
  y: number;
  size: number;
  round?: number;
}

export const KF_BLEND_MS = 350;

export function camKfRange(moves: CameraMove[]): [number, number] | null {
  if (moves.length === 0) return null;
  let first = Infinity,
    last = -Infinity;
  for (const m of moves) {
    if (m.t_ms < first) first = m.t_ms;
    if (m.t_ms > last) last = m.t_ms;
  }
  return [first, last];
}

export function shapeRound(shape: string, roundness: number): number | null {
  if (shape === "circle") return 0.5;
  if (shape === "rect") return 0;
  if (shape === "rounded") return Math.min(0.5, Math.max(0, roundness));
  return null;
}

export function camMoveAt(moves: CameraMove[], t: number, live?: CamPose | null): CamPose | null {
  const ks = [...moves].sort((a, b) => a.t_ms - b.t_ms);
  if (ks.length === 0) return null;
  const first = ks[0],
    last = ks[ks.length - 1];
  const entry = Math.max(0, first.t_ms - KF_BLEND_MS);
  if (t < entry || t > last.t_ms + KF_BLEND_MS) return null;
  const pose = (k: CameraMove): CamPose => ({
    x: k.x,
    y: k.y,
    size: k.size,
    round: shapeRound(k.shape, k.roundness) ?? live?.round,
  });
  if (t < first.t_ms) {
    const win = first.t_ms - entry;
    if (!live || win <= 0) return pose(first);
    return mix(live, pose(first), ease(first.easing, (t - entry) / win));
  }
  if (t > last.t_ms) {
    if (!live) return pose(last);
    return mix(pose(last), live, ease(last.easing, (t - last.t_ms) / KF_BLEND_MS));
  }
  if (t >= last.t_ms) return pose(last);

  const bi = ks.findIndex((k) => k.t_ms > t);
  const a = ks[bi - 1],
    b = ks[bi];
  if (b.t_ms === a.t_ms) return pose(b);
  return mix(pose(a), pose(b), ease(b.easing, (t - a.t_ms) / (b.t_ms - a.t_ms)));
}

function mix(a: CamPose, b: CamPose, f: number): CamPose {
  const round =
    a.round !== undefined && b.round !== undefined ? a.round + (b.round - a.round) * f : (a.round ?? b.round);
  return { x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, size: a.size + (b.size - a.size) * f, round };
}

export function rectFromCenter(
  p: CamPose,
  ow: number,
  oh: number,
  aspect: number,
): [number, number, number, number] {
  const h = p.size * oh;
  const w = h * Math.max(aspect, 0.01);
  return [(p.x * ow - w / 2) / ow, (p.y * oh - h / 2) / oh, w / ow, h / oh];
}

export function camAspect(
  cam: [number, number, number, number, ...number[]],
  ow: number,
  oh: number,
): number {
  return (cam[2] * ow) / Math.max(cam[3] * oh, 0.001);
}

export function liveCamPose(
  cam: [number, number, number, number, number, ...number[]],
  ow: number,
  oh: number,
): CamPose {
  const short = Math.max(Math.min(cam[2] * ow, cam[3] * oh), 0.001);
  return { x: cam[0] + cam[2] / 2, y: cam[1] + cam[3] / 2, size: cam[3], round: (cam[4] * ow) / short };
}

export function radiusScaleForResize(oldH: number, newH: number): number {
  return newH / Math.max(oldH, 0.001);
}

export function overrideCamPanel(
  baseCam: [number, number, number, number, number, number, number, number, number],
  p: CamPose,
  ow: number,
  oh: number,
): [number, number, number, number, number, number, number, number, number] {
  const newRect = rectFromCenter(p, ow, oh, camAspect(baseCam, ow, oh));
  const scale = radiusScaleForResize(baseCam[3], newRect[3]);
  const radius =
    p.round != null ? (p.round * Math.min(newRect[2] * ow, newRect[3] * oh)) / ow : baseCam[4] * scale;
  return [...newRect, radius, baseCam[5] * scale, baseCam[6], baseCam[7], baseCam[8]];
}

export function cameraMovesKey(moves: CameraMove[]): string {
  return moves
    .map((m) => `${m.id}:${m.t_ms}:${m.x}:${m.y}:${m.size}:${m.easing}:${m.shape}:${m.roundness}`)
    .join("|");
}
