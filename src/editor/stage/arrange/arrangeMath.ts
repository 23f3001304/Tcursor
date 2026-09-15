import type { EditOp, LayoutSeg, PanelPose } from "../../../shared/edit";
import type { LayoutPresets, PanelRectDto } from "../../../shared/ipc";
import { radiusScaleForResize } from "../camera/cameraMoves";

export type PanelKind = "screen" | "cam";
export type Corner = "tl" | "tr" | "bl" | "br";
export type Rect = [number, number, number, number];
export interface Panels {
  screen: PanelRectDto;
  cam: PanelRectDto;
}

export interface DragOut {
  pose: PanelPose;
  guideX: number | null;
  guideY: number | null;
}

export type SnapTol = [number, number];

export const MIN_SIZE = 0.05,
  MAX_SIZE = 1.5;

export const SNAP_PX = 8;

const HAIRLINE_PX = 0.5;
const SAFE = 0.045;

export const SNAP_TARGETS = [SAFE, 1 / 3, 0.5, 2 / 3, 1 - SAFE];

export const VISIBLE_ALPHA = 0.004;

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
const clampSize = (v: number) => Math.min(MAX_SIZE, Math.max(MIN_SIZE, v));

export const poseOfRect = (r: Rect): PanelPose => ({ cx: r[0] + r[2] / 2, cy: r[1] + r[3] / 2, size: r[3] });

export const rectAspect = (r: Rect, ow: number, oh: number): number =>
  (r[2] * ow) / Math.max(r[3] * oh, 1e-6);

function nearestTarget(v: number, tol: number): [number, number] | null {
  let best: [number, number] | null = null;
  for (const t of SNAP_TARGETS) {
    const d = t - v;
    if (Math.abs(d) <= tol && (!best || Math.abs(d) < Math.abs(best[1]))) best = [t, d];
  }
  return best;
}

function snapAxis(c: number, len: number, tol: number): number {
  let best: number | null = null;
  for (const off of [-len / 2, 0, len / 2]) {
    const hit = nearestTarget(c + off, tol);
    if (hit && (best === null || Math.abs(hit[1]) < Math.abs(best))) best = hit[1];
  }
  return best === null ? c : c + best;
}

function guideFor(c: number, len: number, tol: number): number | null {
  for (const off of [-len / 2, 0, len / 2]) {
    const hit = nearestTarget(c + off, tol);
    if (hit) return hit[0];
  }
  return null;
}

export function movedPose(
  start: PanelPose,
  d: [number, number],
  aspect: number,
  ow: number,
  oh: number,
  snap: SnapTol | null,
): DragOut {
  let cx = clamp01(start.cx + d[0]),
    cy = clamp01(start.cy + d[1]);
  if (!snap) return { pose: { ...start, cx, cy }, guideX: null, guideY: null };
  const h = start.size,
    w = (h * oh * aspect) / ow;
  cx = clamp01(snapAxis(cx, w, snap[0]));
  cy = clamp01(snapAxis(cy, h, snap[1]));
  return {
    pose: { ...start, cx, cy },
    guideX: guideFor(cx, w, HAIRLINE_PX / ow),
    guideY: guideFor(cy, h, HAIRLINE_PX / oh),
  };
}

function anchorOf(r: Rect, c: Corner): { ax: number; ay: number; sx: number; sy: number } {
  const right = c === "tr" || c === "br",
    bottom = c === "bl" || c === "br";
  return {
    ax: right ? r[0] : r[0] + r[2],
    ay: bottom ? r[1] : r[1] + r[3],
    sx: right ? 1 : -1,
    sy: bottom ? 1 : -1,
  };
}

export function resizedPose(
  r: Rect,
  c: Corner,
  ptr: [number, number],
  aspect: number,
  ow: number,
  oh: number,
  snap: SnapTol | null,
): DragOut {
  const { ax, ay, sx, sy } = anchorOf(r, c);
  const hyp = Math.hypot(aspect, 1);
  const vx = sx * (ptr[0] - ax) * ow,
    vy = sy * (ptr[1] - ay) * oh;
  let hpx = Math.max(0, (vx * aspect + vy) / hyp) / hyp;
  if (snap) hpx = snapResizeHeight(hpx, { ax, ay, sx, sy }, aspect, ow, oh, snap) ?? hpx;
  const size = clampSize(hpx / oh);
  const hf = size,
    wf = (size * oh * aspect) / ow;
  const pose = { cx: clamp01(ax + (sx * wf) / 2), cy: clamp01(ay + (sy * hf) / 2), size };
  const g = snap ? resizeGuides(pose, wf, hf, sx, sy, ow, oh) : { guideX: null, guideY: null };
  return { pose, ...g };
}

function snapResizeHeight(
  hpx: number,
  a: { ax: number; ay: number; sx: number; sy: number },
  aspect: number,
  ow: number,
  oh: number,
  tol: SnapTol,
): number | null {
  const cornerX = a.ax + (a.sx * (hpx * aspect)) / ow,
    cornerY = a.ay + (a.sy * hpx) / oh;
  const hitX = nearestTarget(cornerX, tol[0]),
    hitY = nearestTarget(cornerY, tol[1]);
  const candX = hitX ? (Math.abs(hitX[0] - a.ax) * ow) / aspect : null;
  const candY = hitY ? Math.abs(hitY[0] - a.ay) * oh : null;
  if (candX !== null && candY !== null) return Math.abs(candX - hpx) <= Math.abs(candY - hpx) ? candX : candY;
  return candX ?? candY;
}

function resizeGuides(
  p: PanelPose,
  wf: number,
  hf: number,
  sx: number,
  sy: number,
  ow: number,
  oh: number,
): { guideX: number | null; guideY: number | null } {
  const edgeX = p.cx + (sx * wf) / 2,
    edgeY = p.cy + (sy * hf) / 2;
  return {
    guideX: nearestTarget(edgeX, HAIRLINE_PX / ow)?.[0] ?? null,
    guideY: nearestTarget(edgeY, HAIRLINE_PX / oh)?.[0] ?? null,
  };
}

export function draftPanels(base: Panels, panel: PanelKind, rect: Rect): Panels {
  if (panel === "screen") return { ...base, screen: { ...base.screen, rect, alpha: 1 } };
  const m = radiusScaleForResize(base.cam.rect[3], rect[3]);
  return {
    ...base,
    cam: { ...base.cam, rect, alpha: 1, radius: base.cam.radius * m, ring_px: base.cam.ring_px * m },
  };
}

export function withDraftSeg(presets: LayoutPresets, segId: string, p: Panels): LayoutPresets {
  const entry = { id: segId, screen: p.screen, cam: p.cam };
  const has = presets.segs.some((e) => e.id === segId);
  return {
    ...presets,
    segs: has ? presets.segs.map((e) => (e.id === segId ? entry : e)) : [...presets.segs, entry],
  };
}

export function setArrangementOp(
  seg: LayoutSeg,
  base: Panels,
  panel: PanelKind,
  pose: PanelPose | null,
): EditOp {
  const other: PanelKind = panel === "screen" ? "cam" : "screen";
  const r = base[other];
  const fill = seg.arrangement ? undefined : r.alpha > VISIBLE_ALPHA ? poseOfRect(r.rect) : null;
  const screen = panel === "screen" ? pose : fill;
  const cam = panel === "cam" ? pose : fill;
  return {
    op: "set_arrangement",
    id: seg.id,
    ...(screen !== undefined ? { screen } : {}),
    ...(cam !== undefined ? { cam } : {}),
  };
}
