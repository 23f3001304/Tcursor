import { useEffect, useRef, useState, type RefObject } from "react";
import type { LayoutPresets, PreviewLayout } from "../../lib/ipc";
import type { CameraMove, LayoutSeg } from "../../lib/edit";
import { camAspect, cameraMovesKey, camMoveAt, rectFromCenter, type CamPose } from "./cameraMoves";
import { mapPointerToCamFraction } from "./camDragMapper";
import { layoutAt } from "../timeline/layoutTrack";
import { isNaturalPlaybackTick } from "./playbackTick";
import { attachPointerGesture } from "./pointerGesture";
import { pastDragThreshold } from "../hooks/dragThreshold";

export function CamDragHandle({ layout, layoutPresets, layoutSegs, cameraMoves, timeMs, playing, canvasW, canvasH, canvasRef, camDraftRef, dirtyRef }: {
  layout: PreviewLayout | null; layoutPresets: LayoutPresets | null; layoutSegs: LayoutSeg[];
  cameraMoves: CameraMove[]; timeMs: number;
  /** `Stage`'s own `playing` prop, passed straight through - needed to tell an ordinary playback
   *  tick apart from a real seek (`isNaturalPlaybackTick`, M6) for the `[timeMs]` effect below. */
  playing: boolean;
  canvasW: number; canvasH: number;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  camDraftRef: RefObject<CamPose | null>; dirtyRef: RefObject<boolean>;
}) {
  // dragPose (state) mirrors camDraftRef to drive the handle's CSS position; the rAF loop reads
  // the ref. Stage clears the ref when the playhead moves, and this component unmounts with Move
  // mode, so the local state can never outlive the draft it mirrors.
  const [dragPose, setDragPose] = useState<CamPose | null>(null);
  const detachRef = useRef<(() => void) | null>(null);
  const playingRef = useRef(playing); playingRef.current = playing; // read fresh inside the [timeMs]-only effect below
  const lastMsRef = useRef(timeMs);
  // Moving the playhead discards the unsaved draft - the PiP resets to its sampled pose - UNLESS
  // this `timeMs` change is just an ordinary playback tick, not a real seek/scrub (M6, review
  // round 1 Important 3 - mirrors Stage.tsx's matching effect over `camDraftRef`; see its comment).
  useEffect(() => {
    if (!isNaturalPlaybackTick(lastMsRef.current, timeMs, playingRef.current)) setDragPose(null);
    lastMsRef.current = timeMs;
  }, [timeMs]);
  // The OTHER clear trigger: an explicit action (CameraPanel's "Add keyframe" button) consuming
  // the draft, which writes a real `camera_moves` entry - CameraPanel clears `camDraftRef` itself
  // (the rAF loop's copy) when it does; this clears the LOCAL mirror driving the handle's own CSS
  // position. Gated on `cameraMovesKey` (a CONTENT signature), NOT the `cameraMoves` reference
  // itself (review round 2, Important): `applyEditOp` hands back a brand-new `camera_moves` array
  // reference on EVERY edit routed through it - add a zoom, delete, trim, an AI step - not just
  // camera-move ones, so keying on the reference cleared the mirror on totally unrelated edits
  // too. Repro that fixed: Move-mode drag the PiP (draft uncommitted), press Z to add a zoom - the
  // reference-keyed effect nulled `dragPose`, snapping the drag-handle overlay to `sampledPose`
  // while the composited canvas (still reading the un-cleared `camDraftRef.current`) kept drawing
  // the actual drag position - two on-screen elements visibly disagreeing.
  const cameraMovesKeyRef = useRef(cameraMovesKey(cameraMoves));
  useEffect(() => {
    const key = cameraMovesKey(cameraMoves);
    if (key !== cameraMovesKeyRef.current) setDragPose(null);
    cameraMovesKeyRef.current = key;
  }, [cameraMoves]);
  // Release this drag's window listeners if the handle unmounts mid-gesture (Move mode toggled
  // off) - the old code only ever removed them from its own `up` callback, so an unmount here
  // left `pointermove`/`pointerup`/`pointercancel` bound to `window` forever (L4).
  useEffect(() => () => detachRef.current?.(), []);

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
  // never adds/updates a keyframe - the PiP just shows its sampled pose at the playhead.
  // `attachPointerGesture` (pointercancel + unmount-safe via `detachRef`, L4) keeps the drag alive
  // past the handle; live feedback goes into camDraftRef (read by the rAF loop each frame),
  // dragPose (state) drives the handle's CSS.
  const onHandlePointerDown = (e: React.PointerEvent) => {
    e.stopPropagation();
    const c = canvasRef.current; if (!c) return;
    // Outside the keyframe span `sampledPose` is null, so the drag starts from the LIVE pose -
    // i.e. the size the panel is actually showing right now, not a stale keyframe's.
    const start = { x: e.clientX, y: e.clientY, size: (camDraftRef.current ?? sampledPose ?? livePose)?.size ?? 0.25 };
    let moved = false;
    const move = (ev: PointerEvent) => {
      if (!moved) {
        if (!pastDragThreshold(ev.clientX - start.x, ev.clientY - start.y, 4)) return; // ignore a click
        moved = true;
      }
      const [x, y] = mapPointerToCamFraction({ clientX: ev.clientX, clientY: ev.clientY, canvasElement: c });
      // Update the live draft only - NO commit. Saving is the Camera panel's Update/Add button,
      // which also clears camDraftRef itself once it does (CameraPanel.tsx's addKeyframeHere).
      camDraftRef.current = { x, y, size: start.size }; setDragPose({ x, y, size: start.size }); dirtyRef.current = true;
    };
    detachRef.current = attachPointerGesture(move, () => { detachRef.current = null; });
  };

  if (!pipRect) return null;
  return (
    <div className={`e-camdrag${dragPose ? " drag" : ""}`}
      style={{ left: `${pipRect[0] * 100}%`, top: `${pipRect[1] * 100}%`, width: `${pipRect[2] * 100}%`, height: `${pipRect[3] * 100}%`, borderRadius: `${handleRadiusPct}%` }}
      title="Drag to reposition the webcam"
      onPointerDown={onHandlePointerDown} />
  );
}
