import { useMemo, type RefObject } from "react";
import { maskFrameAt } from "./maskFrame";
import { useMaskDrag } from "./useMaskDrag";
import type { StageProps } from "../stageProps";

export function useStageMask(
  p: StageProps,
  canvasW: number,
  canvasH: number,
  tOut: number,
  dirtyRef: RefObject<boolean>,
  arranging: boolean,
) {
  const selMask = p.effects.find((e) => e.id === p.sel && e.kind !== "spotlight") ?? null;
  const off = p.moveMode || arranging || p.aimMode;
  // INVARIANT: the drag pose passed here is null because the overlay is off in exactly the two
  // states that make the painter's `activeCamDraft` non-null - move mode and arranging, both in
  // `off` above. Anything else in this frame must stay identical to drawCompositeFrame's.
  const frame = useMemo(
    () => maskFrameAt(p, tOut, canvasW, canvasH, null),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      p.track,
      p.layout,
      p.layoutPresets,
      p.layoutSegs,
      p.cameraMoves,
      p.zooms,
      p.zoomSettings,
      tOut,
      canvasW,
      canvasH,
    ],
  );
  return useMaskDrag({
    effect: off ? null : selMask,
    layout: frame.layout,
    cam: frame.cam,
    canvasW,
    canvasH,
    tOut,
    defaultDim: p.clickfx.spotlight_dim,
    dirtyRef,
    onCommit: (id, rect) => void p.onApply({ op: "update_effect", id, rect }),
  });
}
