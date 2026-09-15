import { useEffect, useRef, useState, type RefObject } from "react";
import type { LayoutPresets, PreviewLayout } from "../../../shared/ipc";
import type { CameraMove, LayoutSeg } from "../../../shared/edit";
import { cameraMovesKey, camMoveAt, liveCamPose, overrideCamPanel, type CamPose } from "./cameraMoves";
import { mapPointerToCamFraction } from "./camDragMapper";
import { layoutAt } from "../../timeline/model/layoutTrack";
import { isNaturalPlaybackTick } from "../transport/playback";
import { attachPointerGesture } from "../pointerGesture";
import { pastDragThreshold } from "../../util/dragThreshold";

export function CamDragHandle({
  layout,
  layoutPresets,
  layoutSegs,
  cameraMoves,
  timeMs,
  playing,
  canvasW,
  canvasH,
  canvasRef,
  camDraftRef,
  dirtyRef,
}: {
  layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null;
  layoutSegs: LayoutSeg[];
  cameraMoves: CameraMove[];
  timeMs: number;
  playing: boolean;
  canvasW: number;
  canvasH: number;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  camDraftRef: RefObject<CamPose | null>;
  dirtyRef: RefObject<boolean>;
}) {
  const [dragPose, setDragPose] = useState<CamPose | null>(() => camDraftRef.current);
  const detachRef = useRef<(() => void) | null>(null);
  const playingRef = useRef(playing);
  playingRef.current = playing;
  const lastMsRef = useRef(timeMs);
  useEffect(() => {
    if (!isNaturalPlaybackTick(lastMsRef.current, timeMs, playingRef.current)) setDragPose(null);
    lastMsRef.current = timeMs;
  }, [timeMs]);
  const cameraMovesKeyRef = useRef(cameraMovesKey(cameraMoves));
  useEffect(() => {
    const key = cameraMovesKey(cameraMoves);
    if (key !== cameraMovesKeyRef.current) setDragPose(null);
    cameraMovesKeyRef.current = key;
  }, [cameraMoves]);
  useEffect(() => () => detachRef.current?.(), []);

  const baseLayout = layoutAt(layoutSegs, layoutPresets, timeMs, [canvasW, canvasH]) ?? layout;
  const livePose = baseLayout?.cam ? liveCamPose(baseLayout.cam, canvasW, canvasH) : null;
  const sampledPose = camMoveAt(cameraMoves, timeMs, livePose);
  const activePose = dragPose ?? sampledPose;
  const panel = !baseLayout?.cam
    ? null
    : activePose
      ? overrideCamPanel(baseLayout.cam, activePose, canvasW, canvasH)
      : baseLayout.cam;
  const handleRadiusPct = panel && panel[2] > 0 ? Math.min(50, (panel[4] / panel[2]) * 100) : 12;

  const onHandlePointerDown = (e: React.PointerEvent) => {
    e.stopPropagation();
    const c = canvasRef.current;
    if (!c) return;
    const start = {
      x: e.clientX,
      y: e.clientY,
      size: (camDraftRef.current ?? sampledPose ?? livePose)?.size ?? 0.25,
    };
    let moved = false;
    const move = (ev: PointerEvent) => {
      if (!moved) {
        if (!pastDragThreshold(ev.clientX - start.x, ev.clientY - start.y, 4)) return;
        moved = true;
      }
      const [x, y] = mapPointerToCamFraction({ clientX: ev.clientX, clientY: ev.clientY, canvasElement: c });
      camDraftRef.current = { x, y, size: start.size };
      setDragPose({ x, y, size: start.size });
      dirtyRef.current = true;
    };
    detachRef.current = attachPointerGesture(move, () => {
      detachRef.current = null;
    });
  };

  if (!panel) return null;
  return (
    <div
      className={`e-camdrag${dragPose ? " drag" : ""}`}
      style={{
        left: `${panel[0] * 100}%`,
        top: `${panel[1] * 100}%`,
        width: `${panel[2] * 100}%`,
        height: `${panel[3] * 100}%`,
        borderRadius: `${handleRadiusPct}%`,
      }}
      title="Drag to reposition the webcam"
      onPointerDown={onHandlePointerDown}
    />
  );
}
