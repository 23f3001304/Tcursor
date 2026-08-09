import { useEffect, useState, type RefObject } from "react";
import type { LayoutPresets, PreviewLayout } from "../../lib/ipc";
import type { CameraMove, LayoutSeg } from "../../lib/edit";
import { camAspect, camMoveAt, rectFromCenter, type CamPose } from "./cameraMoves";
import { mapPointerToCamFraction } from "./camDragMapper";
import { layoutAt } from "../timeline/layoutTrack";

export function CamDragHandle({ layout, layoutPresets, layoutSegs, cameraMoves, timeMs, canvasW, canvasH, canvasRef, camDraftRef, dirtyRef }: {
  layout: PreviewLayout | null; layoutPresets: LayoutPresets | null; layoutSegs: LayoutSeg[];
  cameraMoves: CameraMove[]; timeMs: number; canvasW: number; canvasH: number;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  camDraftRef: RefObject<CamPose | null>; dirtyRef: RefObject<boolean>;
}) {
  // dragPose (state) mirrors camDraftRef to drive the handle's CSS position; the rAF loop reads
  // the ref. Stage clears the ref when the playhead moves, and this component unmounts with Move
  // mode, so the local state can never outlive the draft it mirrors.
  const [dragPose, setDragPose] = useState<CamPose | null>(null);
  // Moving the playhead discards the unsaved draft - the PiP resets to its sampled pose. (Stage
  // owns the matching `camDraftRef` reset, which must happen even with Move mode off.)
  useEffect(() => { setDragPose(null); }, [timeMs]);

  const baseLayout = layoutAt(layoutSegs, layoutPresets, timeMs, [canvasW, canvasH]) ?? layout;
  // The LIVE (un-overridden, layout-resolved) PiP pose the keyframe track eases to and from at
  // its span edges - same derivation useCompositeLoop.ts makes, so the handle matches the drawn
  // frame. Outside the span `camMoveAt` is null and this pose IS what's on screen.
  const livePose = baseLayout?.cam
    ? { x: baseLayout.cam[0] + baseLayout.cam[2] / 2, y: baseLayout.cam[1] + baseLayout.cam[3] / 2, size: baseLayout.cam[3] }
    : null;
  const sampledPose = camMoveAt(cameraMoves, timeMs, livePose);
  const activePose = dragPose ?? sampledPose;
  const pipRect: [number, number, number, number] | null = !baseLayout?.cam ? null
    : activePose ? rectFromCenter(activePose, canvasW, canvasH, camAspect(baseLayout.cam, canvasW, canvasH))
    : [baseLayout.cam[0], baseLayout.cam[1], baseLayout.cam[2], baseLayout.cam[3]];
  // Match the handle's rounding to the webcam shape (radius/width ratio: ~50% circle, frac rounded,
  // 0 rect) so it hugs the PiP instead of a boxy border sticking out past a round webcam.
  const handleRadiusPct = baseLayout?.cam && baseLayout.cam[2] > 0
    ? Math.min(50, (baseLayout.cam[4] / baseLayout.cam[2]) * 100) : 12;

  // Move-mode drag: a pointerdown on the handle records the start but does NOT move or commit
  // anything - the PiP only follows (and a keyframe is only written) once the pointer actually
  // drags past a small threshold. So a plain click on the preview, or scrubbing the playhead,
  // never adds/updates a keyframe - the PiP just shows its sampled pose at the playhead. Window
  // listeners (not React pointer capture) keep the drag alive past the handle; live feedback goes
  // into camDraftRef (read by the rAF loop each frame), dragPose (state) drives the handle's CSS.
  const onHandlePointerDown = (e: React.PointerEvent) => {
    e.stopPropagation();
    const c = canvasRef.current; if (!c) return;
    // Outside the keyframe span `sampledPose` is null, so the drag starts from the LIVE pose -
    // i.e. the size the panel is actually showing right now, not a stale keyframe's.
    const start = { x: e.clientX, y: e.clientY, size: (camDraftRef.current ?? sampledPose ?? livePose)?.size ?? 0.25 };
    let moved = false;
    const move = (ev: PointerEvent) => {
      if (!moved && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) < 4) return; // ignore a click
      moved = true;
      const [x, y] = mapPointerToCamFraction({ clientX: ev.clientX, clientY: ev.clientY, canvasElement: c });
      // Update the live draft only - NO commit. Saving is the Camera panel's Update/Add button.
      camDraftRef.current = { x, y, size: start.size }; setDragPose({ x, y, size: start.size }); dirtyRef.current = true;
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  if (!pipRect) return null;
  return (
    <div className={`e-camdrag${dragPose ? " drag" : ""}`}
      style={{ left: `${pipRect[0] * 100}%`, top: `${pipRect[1] * 100}%`, width: `${pipRect[2] * 100}%`, height: `${pipRect[3] * 100}%`, borderRadius: `${handleRadiusPct}%` }}
      title="Drag to reposition the webcam"
      onPointerDown={onHandlePointerDown} />
  );
}
