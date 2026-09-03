import { useEffect, useRef, useState, type RefObject } from "react";
import type { EditDoc, EditOp, LayoutSeg, PanelPose } from "../../../lib/edit";
import type { LayoutPresets } from "../../../lib/ipc";
import { resolvedPanelsFor } from "../../timeline/layoutTrack";
import { rectFromCenter } from "../cameraMoves";
import { mapPointerToCamFraction } from "../camDragMapper";
import { attachPointerGesture } from "../pointerGesture";
import { pastDragThreshold } from "../../hooks/dragThreshold";
import {
  draftPanels, movedPose, poseOfRect, rectAspect, resizedPose, setArrangementOp, withDraftSeg,
  SNAP_PX, type Corner, type PanelKind, type Panels, type SnapTol,
} from "./arrangeMath";

/** The whole resolved pair, not just the panel under the pointer: dragging the screen and then the
 *  cam before the first commit's refetch has landed must not drop the screen's new position. */
interface Draft { panels: Panels; guideX: number | null; guideY: number | null }

/** The arrange-mode gesture: a live pose draft the composite loop draws, committed on release as
 *  ONE `set_arrangement` op (the 400ms history coalesce window is what groups a burst of them).
 *
 *  The draft never gets its own drawing path. It is folded straight back into `LayoutPresets.segs`
 *  (`withDraftSeg`) - the per-segment resolved-rect channel L2 already built - so `layoutAt` picks
 *  it up exactly like a committed arrangement, cross-fades included, and `Stage` only has to hand
 *  the loop the merged presets instead of the raw ones. Same "presentation-only, committed by an
 *  explicit action" contract as `camDraftRef`.
 *
 *  The draft is dropped when the segment's RESOLVED entry changes (`presets.segs` reference for
 *  this id), i.e. once the refetch that the commit triggered has landed - not when the doc first
 *  comes back. Clearing on the doc would snap the panel back to its pre-drag rect for the few
 *  frames before `preview_layouts` returns; keying on the resolved entry hands over seamlessly. */
export function useArrangeDrag({ seg, presets, canvasRef, canvasW, canvasH, dirtyRef, onApply }: {
  seg: LayoutSeg | null; presets: LayoutPresets | null;
  canvasRef: RefObject<HTMLCanvasElement | null>; canvasW: number; canvasH: number;
  dirtyRef: RefObject<boolean>; onApply: (op: EditOp) => Promise<EditDoc | null>;
}) {
  const [draft, setDraft] = useState<Draft | null>(null);
  const [active, setActive] = useState<PanelKind | null>(null);
  const detachRef = useRef<(() => void) | null>(null);
  useEffect(() => () => detachRef.current?.(), []); // unmount mid-gesture must not leak listeners

  const base: Panels | null = seg && presets ? resolvedPanelsFor(seg, presets) : null;
  const segEntry = seg && presets ? presets.segs.find((e) => e.id === seg.id) ?? null : null;
  useEffect(() => { setDraft(null); }, [segEntry, seg?.id]);
  // What is ON SCREEN right now - the draft while one is live, else the resolved panels. Every
  // gesture starts from THIS, not from `base`: a second drag begun before the previous commit's
  // refetch has landed would otherwise snap the panel back to its pre-drag rect first.
  const panels = draft?.panels ?? base;

  const onPanelDown = (e: React.PointerEvent, panel: PanelKind, corner: Corner | null) => {
    e.stopPropagation();
    const c = canvasRef.current, b = panels, s = seg;
    if (!c || !b || !s) return;
    const rect = b[panel].rect;
    const aspect = rectAspect(rect, canvasW, canvasH);
    const startPose = poseOfRect(rect);
    const sx = e.clientX, sy = e.clientY;
    const from = mapPointerToCamFraction({ clientX: sx, clientY: sy, canvasElement: c });
    // "8 stage-px" means 8 pixels ON SCREEN, so the threshold is converted against the canvas'
    // DISPLAYED rect, not its backing store (CSS scales the stage to fit). Measured once per
    // gesture - the stage cannot resize mid-drag, and this keeps it off the pointermove path.
    const dr = c.getBoundingClientRect();
    const tol: SnapTol = [SNAP_PX / Math.max(dr.width, 1), SNAP_PX / Math.max(dr.height, 1)];
    let moved = false, last: PanelPose | null = null;
    setActive(panel);
    const move = (ev: PointerEvent) => {
      if (!moved) {
        if (!pastDragThreshold(ev.clientX - sx, ev.clientY - sy)) return; // a click must not commit
        moved = true;
      }
      const p = mapPointerToCamFraction({ clientX: ev.clientX, clientY: ev.clientY, canvasElement: c });
      const snap = ev.altKey ? null : tol; // Alt suspends snapping, the convention every other snapping drag uses
      const out = corner
        ? resizedPose(rect, corner, p, aspect, canvasW, canvasH, snap)
        : movedPose(startPose, [p[0] - from[0], p[1] - from[1]], aspect, canvasW, canvasH, snap);
      last = out.pose;
      const r = rectFromCenter({ x: out.pose.cx, y: out.pose.cy, size: out.pose.size }, canvasW, canvasH, aspect);
      setDraft((d) => ({ panels: draftPanels(d?.panels ?? b, panel, r), guideX: out.guideX, guideY: out.guideY }));
      dirtyRef.current = true; // the paused loop only recomposites on a dirty flag
    };
    detachRef.current = attachPointerGesture(move, () => {
      detachRef.current = null;
      setActive(null);
      setDraft((d) => (d ? { ...d, guideX: null, guideY: null } : d)); // guides belong to the gesture
      if (moved && last) void onApply(setArrangementOp(s, b, panel, last));
    });
  };

  const onHideCam = () => { if (panels && seg) void onApply(setArrangementOp(seg, panels, "cam", null)); };

  return {
    panels,
    /** `presets` with this drag's draft folded in - what `Stage` hands the composite loop. */
    presets: presets && seg && draft && panels ? withDraftSeg(presets, seg.id, panels) : presets,
    guideX: draft?.guideX ?? null, guideY: draft?.guideY ?? null, active, onPanelDown, onHideCam,
  };
}
