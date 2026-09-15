import type { RefObject } from "react";
import type { CamSample, PreviewLayout } from "../../shared/ipc";
import { camAt } from "./camera/camera";
import { outOf, type TimeMap } from "../../shared/math/remap";
import { useReticleDrag } from "./useReticleDrag";
import { mapCanvasClickToZoomTarget, mapZoomTargetToCanvasPoint } from "./camera/zoomTargetMapper";

export function useStagePointer({
  canvasRef,
  screenRef,
  layoutRef,
  trackRef,
  mapRef,
  timeRef,
  arranging,
  aimMode,
  aimPoint,
  playing,
  canvasW,
  canvasH,
  layout,
  track,
  tOut,
  onZoomAt,
  onAimAt,
}: {
  canvasRef: RefObject<HTMLCanvasElement | null>;
  screenRef: RefObject<HTMLVideoElement | null>;
  layoutRef: RefObject<PreviewLayout | null>;
  trackRef: RefObject<CamSample[]>;
  mapRef: RefObject<TimeMap>;
  timeRef: RefObject<number>;
  arranging: boolean;
  aimMode: boolean;
  aimPoint: [number, number] | null;
  playing: boolean;
  canvasW: number;
  canvasH: number;
  layout: PreviewLayout | null;
  track: CamSample[];
  tOut: number;
  onZoomAt: (x: number, y: number) => void;
  onAimAt: (x: number, y: number) => void;
}) {
  const targetUnderPointer = (clientX: number, clientY: number) => {
    const c = canvasRef.current,
      sv = screenRef.current;
    if (!c || !sv) return null;
    if (sv.videoWidth <= 0 || sv.videoHeight <= 0) return null;
    return mapCanvasClickToZoomTarget({
      clientX,
      clientY,
      canvasElement: c,
      layout: layoutRef.current,
      cam: camAt(trackRef.current, outOf(mapRef.current, timeRef.current)),
    });
  };
  const onCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (arranging) return;
    const t = targetUnderPointer(e.clientX, e.clientY);
    if (t) (aimMode ? onAimAt : onZoomAt)(t[0], t[1]);
  };
  const { liveAim, aimDrag, onReticleDown } = useReticleDrag(aimPoint, onAimAt, targetUnderPointer);
  const shownAim = liveAim ?? aimPoint;
  const reticle =
    shownAim && !playing && !arranging
      ? mapZoomTargetToCanvasPoint({
          tx: shownAim[0],
          ty: shownAim[1],
          canvasW,
          canvasH,
          layout,
          cam: camAt(track, tOut),
        })
      : null;

  return { targetUnderPointer, onCanvasClick, reticle, aimDrag, onReticleDown };
}
