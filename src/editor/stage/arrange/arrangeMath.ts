import type { EditOp, LayoutSeg, PanelPose } from "../../../lib/edit";
import type { LayoutPresets, PanelRectDto } from "../../../lib/ipc";
import { radiusScaleForResize } from "../cameraMoves";

export type PanelKind = "screen" | "cam";
export type Corner = "tl" | "tr" | "bl" | "br";
export type Rect = [number, number, number, number];
export interface Panels { screen: PanelRectDto; cam: PanelRectDto }
/** A drag result: the pose to preview/commit plus the two guide lines (frame fractions) that are
 *  currently snapped, `null` on an axis that isn't. */
export interface DragOut { pose: PanelPose; guideX: number | null; guideY: number | null }
/** Per-axis snap tolerance, as FRACTIONS of the output frame - `null` means snapping is off (Alt).
 *  `useArrangeDrag` derives it once per gesture as `SNAP_PX / displayed width|height`. */
export type SnapTol = [number, number];

// Mirrors the Rust op clamps (edit/ops/arrangement.rs) so a drag can never build a pose the
// backend would silently clamp out from under the live preview.
export const MIN_SIZE = 0.05, MAX_SIZE = 1.5;
/** Snap threshold, in STAGE pixels - literally 8 pixels on screen. Callers convert it against the
 *  canvas' DISPLAYED rect (not its backing store) into the per-axis `SnapTol` below, so the feel is
 *  the same 8px whatever resolution the stage is scaled to. */
export const SNAP_PX = 8;
/** How close a line has to be, in OUTPUT pixels, to count as "sitting on" a target when a guide is
 *  re-derived after clamping. Half a pixel: a real snap lands exactly, a clamp does not. */
const HAIRLINE_PX = 0.5;
const SAFE = 0.045; // the 4.5% safe-margin inset
/** Snap lines, as fractions of the output frame - identical set on both axes: the safe margins,
 *  the thirds, and the frame center. */
export const SNAP_TARGETS = [SAFE, 1 / 3, 0.5, 2 / 3, 1 - SAFE];
/** Alpha at/below which a resolved panel counts as hidden - the same threshold `toPreviewLayout`
 *  (timeline/layoutTrack.ts) uses to decide whether to draw the cam at all. */
export const VISIBLE_ALPHA = 0.004;

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
const clampSize = (v: number) => Math.min(MAX_SIZE, Math.max(MIN_SIZE, v));

/** The pose a resolved rect stands for - the inverse of `rect_from_center`, and the ONLY pose
 *  math this module needs beyond the drag deltas themselves: a gesture starts from the rect the
 *  user actually grabbed, so what they drag is what gets committed. */
export const poseOfRect = (r: Rect): PanelPose => ({ cx: r[0] + r[2] / 2, cy: r[1] + r[3] / 2, size: r[3] });
/** A panel's own pixel aspect (w/h) from a resolved rect whose w/h are fractions of DIFFERENT
 *  axes - what `rectFromCenter` needs so a pose (height only) never stretches the panel. */
export const rectAspect = (r: Rect, ow: number, oh: number): number => (r[2] * ow) / Math.max(r[3] * oh, 1e-6);

/** Nearest snap target to `v` within `tol`, as `[target, delta]`, else null. */
function nearestTarget(v: number, tol: number): [number, number] | null {
  let best: [number, number] | null = null;
  for (const t of SNAP_TARGETS) {
    const d = t - v;
    if (Math.abs(d) <= tol && (!best || Math.abs(d) < Math.abs(best[1]))) best = [t, d];
  }
  return best;
}

/** One axis of a MOVE snap: tries the panel's near edge, center and far edge against every
 *  target, and returns the center shifted onto the closest hit. */
function snapAxis(c: number, len: number, tol: number): number {
  let best: number | null = null;
  for (const off of [-len / 2, 0, len / 2]) {
    const hit = nearestTarget(c + off, tol);
    if (hit && (best === null || Math.abs(hit[1]) < Math.abs(best))) best = hit[1];
  }
  return best === null ? c : c + best;
}

/** The target one of the panel's three lines ACTUALLY sits on now - used to re-derive the guides
 *  from the final, clamped pose so a clamp that pulled the panel off a target cannot leave a guide
 *  drawn through a line the panel no longer touches. */
function guideFor(c: number, len: number, tol: number): number | null {
  for (const off of [-len / 2, 0, len / 2]) {
    const hit = nearestTarget(c + off, tol);
    if (hit) return hit[0];
  }
  return null;
}

/** Body drag: the start pose translated by a pointer delta in frame fractions, snapped (unless
 *  `snap` is null - Alt) and clamped to the op's own range. */
export function movedPose(start: PanelPose, d: [number, number], aspect: number, ow: number, oh: number, snap: SnapTol | null): DragOut {
  let cx = clamp01(start.cx + d[0]), cy = clamp01(start.cy + d[1]);
  if (!snap) return { pose: { ...start, cx, cy }, guideX: null, guideY: null };
  const h = start.size, w = (h * oh * aspect) / ow;
  cx = clamp01(snapAxis(cx, w, snap[0]));
  cy = clamp01(snapAxis(cy, h, snap[1]));
  return { pose: { ...start, cx, cy },
    guideX: guideFor(cx, w, HAIRLINE_PX / ow), guideY: guideFor(cy, h, HAIRLINE_PX / oh) };
}

/** The corner opposite `c` (the resize anchor) as a frame-fraction point, plus the outward signs
 *  of the dragged corner relative to it. */
function anchorOf(r: Rect, c: Corner): { ax: number; ay: number; sx: number; sy: number } {
  const right = c === "tr" || c === "br", bottom = c === "bl" || c === "br";
  return { ax: right ? r[0] : r[0] + r[2], ay: bottom ? r[1] : r[1] + r[3], sx: right ? 1 : -1, sy: bottom ? 1 : -1 };
}

/** Corner drag: the panel resized about its OPPOSITE corner, height driven by projecting the
 *  pointer onto the panel's own aspect diagonal - so width always follows the aspect and the
 *  content is never stretched. Snapping moves the dragged corner's edge onto a target (which is a
 *  change of SIZE here, not of center - the anchor is fixed), whichever axis is nearer. */
export function resizedPose(r: Rect, c: Corner, ptr: [number, number], aspect: number, ow: number, oh: number, snap: SnapTol | null): DragOut {
  const { ax, ay, sx, sy } = anchorOf(r, c);
  const hyp = Math.hypot(aspect, 1);
  // Project the anchor->pointer vector (in output px, so the aspect is meaningful) onto the unit
  // diagonal (aspect, 1)/hyp; the corner sits at h*hyp along it, so h = projection / hyp.
  const vx = sx * (ptr[0] - ax) * ow, vy = sy * (ptr[1] - ay) * oh;
  let hpx = Math.max(0, (vx * aspect + vy) / hyp) / hyp;
  if (snap) hpx = snapResizeHeight(hpx, { ax, ay, sx, sy }, aspect, ow, oh, snap) ?? hpx;
  const size = clampSize(hpx / oh);
  const hf = size, wf = (size * oh * aspect) / ow;
  const pose = { cx: clamp01(ax + sx * wf / 2), cy: clamp01(ay + sy * hf / 2), size };
  const g = snap ? resizeGuides(pose, wf, hf, sx, sy, ow, oh) : { guideX: null, guideY: null };
  return { pose, ...g };
}

/** The snapped height (output px) for a resize, or null when neither dragged edge is near a
 *  target: each candidate target is solved back into a height about the fixed anchor, and the one
 *  whose corner lands closest to the unsnapped corner wins. */
function snapResizeHeight(hpx: number, a: { ax: number; ay: number; sx: number; sy: number }, aspect: number, ow: number, oh: number, tol: SnapTol): number | null {
  const cornerX = a.ax + a.sx * (hpx * aspect) / ow, cornerY = a.ay + a.sy * hpx / oh;
  const hitX = nearestTarget(cornerX, tol[0]), hitY = nearestTarget(cornerY, tol[1]);
  const candX = hitX ? Math.abs(hitX[0] - a.ax) * ow / aspect : null;
  const candY = hitY ? Math.abs(hitY[0] - a.ay) * oh : null;
  if (candX !== null && candY !== null) return Math.abs(candX - hpx) <= Math.abs(candY - hpx) ? candX : candY;
  return candX ?? candY;
}

/** Which guide lines a finished resize is actually sitting on - recomputed from the CLAMPED pose
 *  so a size the clamp pulled off its target doesn't keep drawing a guide it no longer touches. */
function resizeGuides(p: PanelPose, wf: number, hf: number, sx: number, sy: number, ow: number, oh: number): { guideX: number | null; guideY: number | null } {
  const edgeX = p.cx + sx * wf / 2, edgeY = p.cy + sy * hf / 2;
  return { guideX: nearestTarget(edgeX, HAIRLINE_PX / ow)?.[0] ?? null, guideY: nearestTarget(edgeY, HAIRLINE_PX / oh)?.[0] ?? null };
}

/** The live-drag panel pair: `base` with ONE panel's rect replaced. The cam's radius/ring scale
 *  by the same height ratio `override_camera` uses (via the existing tested mirror), so a circular
 *  webcam stays round mid-drag; the screen's radius is a fraction of canvas HEIGHT and so does not
 *  scale with its panel (L1's resolution decision, mirrored). Presentation only - the committed
 *  pose is re-resolved by Rust the moment the op lands. */
export function draftPanels(base: Panels, panel: PanelKind, rect: Rect): Panels {
  if (panel === "screen") return { ...base, screen: { ...base.screen, rect, alpha: 1 } };
  const m = radiusScaleForResize(base.cam.rect[3], rect[3]);
  return { ...base, cam: { ...base.cam, rect, alpha: 1, radius: base.cam.radius * m, ring_px: base.cam.ring_px * m } };
}

/** `presets` with one segment's `segs` entry replaced (or appended) by the live draft - the same
 *  per-segment override channel L2 already resolves through, so `layoutAt` cross-fades a dragged
 *  segment exactly as it will once committed, with no draft-specific code path of its own. */
export function withDraftSeg(presets: LayoutPresets, segId: string, p: Panels): LayoutPresets {
  const entry = { id: segId, screen: p.screen, cam: p.cam };
  const has = presets.segs.some((e) => e.id === segId);
  return { ...presets, segs: has ? presets.segs.map((e) => (e.id === segId ? entry : e)) : [...presets.segs, entry] };
}

/** The `set_arrangement` op for changing ONE panel. On a segment that has no arrangement yet the
 *  backend bases the write off "both panels hidden" (L1 decision 3), so the untouched panel is
 *  filled in from what it resolves to RIGHT NOW - its current pose when visible, an explicit
 *  `null` when the provenance preset hides it. That makes a first drag (or a first cam-hide)
 *  preserve the segment's look instead of blanking the other half of the stage. */
export function setArrangementOp(seg: LayoutSeg, base: Panels, panel: PanelKind, pose: PanelPose | null): EditOp {
  const other: PanelKind = panel === "screen" ? "cam" : "screen";
  const r = base[other];
  // `undefined` = leave the key OFF the wire entirely ("don't touch this panel"), which is a
  // different instruction from an explicit `null` ("hide it") - see L1's wire table.
  const fill = seg.arrangement ? undefined : r.alpha > VISIBLE_ALPHA ? poseOfRect(r.rect) : null;
  const screen = panel === "screen" ? pose : fill;
  const cam = panel === "cam" ? pose : fill;
  return {
    op: "set_arrangement", id: seg.id,
    ...(screen !== undefined ? { screen } : {}), ...(cam !== undefined ? { cam } : {}),
  };
}
