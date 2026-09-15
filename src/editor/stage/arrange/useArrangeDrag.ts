import { useEffect, useRef, useState, type RefObject } from "react";
import type { EditDoc, EditOp, LayoutSeg, PanelPose } from "../../../shared/edit";
import type { LayoutPresets } from "../../../shared/ipc";
import { resolvedPanelsFor } from "../../timeline/model/layoutTrack";
import { rectFromCenter } from "../camera/cameraMoves";
import { mapPointerToCamFraction } from "../camera/camDragMapper";
import { attachPointerGesture } from "../pointerGesture";
import { pastDragThreshold } from "../../util/dragThreshold";
import {
  draftPanels,
  movedPose,
  poseOfRect,
  rectAspect,
  resizedPose,
  setArrangementOp,
  withDraftSeg,
  SNAP_PX,
  type Corner,
  type PanelKind,
  type Panels,
  type SnapTol,
} from "./arrangeMath";

interface Draft {
  panels: Panels;
  guideX: number | null;
  guideY: number | null;
}

export function useArrangeDrag({
  seg,
  presets,
  canvasRef,
  canvasW,
  canvasH,
  dirtyRef,
  onApply,
}: {
  seg: LayoutSeg | null;
  presets: LayoutPresets | null;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  canvasW: number;
  canvasH: number;
  dirtyRef: RefObject<boolean>;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
}) {
  const [draft, setDraft] = useState<Draft | null>(null);
  const [active, setActive] = useState<PanelKind | null>(null);
  const detachRef = useRef<(() => void) | null>(null);
  useEffect(() => () => detachRef.current?.(), []);

  const base: Panels | null = seg && presets ? resolvedPanelsFor(seg, presets) : null;
  const segEntry = seg && presets ? (presets.segs.find((e) => e.id === seg.id) ?? null) : null;
  useEffect(() => {
    setDraft(null);
  }, [segEntry, seg?.id]);
  const panels = draft?.panels ?? base;

  const onPanelDown = (e: React.PointerEvent, panel: PanelKind, corner: Corner | null) => {
    e.stopPropagation();
    const c = canvasRef.current,
      b = panels,
      s = seg;
    if (!c || !b || !s) return;
    const rect = b[panel].rect;
    const aspect = rectAspect(rect, canvasW, canvasH);
    const startPose = poseOfRect(rect);
    const sx = e.clientX,
      sy = e.clientY;
    const from = mapPointerToCamFraction({ clientX: sx, clientY: sy, canvasElement: c });
    const dr = c.getBoundingClientRect();
    const tol: SnapTol = [SNAP_PX / Math.max(dr.width, 1), SNAP_PX / Math.max(dr.height, 1)];
    let moved = false,
      last: PanelPose | null = null;
    setActive(panel);
    const move = (ev: PointerEvent) => {
      if (!moved) {
        if (!pastDragThreshold(ev.clientX - sx, ev.clientY - sy)) return;
        moved = true;
      }
      const p = mapPointerToCamFraction({ clientX: ev.clientX, clientY: ev.clientY, canvasElement: c });
      const snap = ev.altKey ? null : tol;
      const out = corner
        ? resizedPose(rect, corner, p, aspect, canvasW, canvasH, snap)
        : movedPose(startPose, [p[0] - from[0], p[1] - from[1]], aspect, canvasW, canvasH, snap);
      last = out.pose;
      const r = rectFromCenter(
        { x: out.pose.cx, y: out.pose.cy, size: out.pose.size },
        canvasW,
        canvasH,
        aspect,
      );
      setDraft((d) => ({
        panels: draftPanels(d?.panels ?? b, panel, r),
        guideX: out.guideX,
        guideY: out.guideY,
      }));
      dirtyRef.current = true;
    };
    detachRef.current = attachPointerGesture(move, () => {
      detachRef.current = null;
      setActive(null);
      setDraft((d) => (d ? { ...d, guideX: null, guideY: null } : d));
      if (moved && last) void onApply(setArrangementOp(s, b, panel, last));
    });
  };

  const onHideCam = () => {
    if (panels && seg) void onApply(setArrangementOp(seg, panels, "cam", null));
  };

  return {
    panels,
    presets: presets && seg && draft && panels ? withDraftSeg(presets, seg.id, panels) : presets,
    guideX: draft?.guideX ?? null,
    guideY: draft?.guideY ?? null,
    active,
    onPanelDown,
    onHideCam,
  };
}
