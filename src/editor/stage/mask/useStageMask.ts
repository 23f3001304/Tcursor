import type { RefObject } from "react";
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
  return useMaskDrag({
    effect: p.moveMode || arranging || p.aimMode ? null : selMask,
    layout: p.layout,
    cam: { cx: 0.5, cy: 0.5, scale: 1 },
    canvasW,
    canvasH,
    tOut,
    defaultDim: p.clickfx.spotlight_dim,
    dirtyRef,
    onCommit: (id, rect) => void p.onApply({ op: "update_effect", id, rect }),
  });
}
