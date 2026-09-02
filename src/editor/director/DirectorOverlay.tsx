import type { RefObject } from "react";
import { AnimatePresence } from "motion/react";
import { DirectorPointer, type DirectorPointerHandle } from "./DirectorPointer";
import { DirectorScrim } from "./DirectorScrim";

/** Everything the AI director's choreographed reveal renders: the fake pointer (mounted only
 *  while `running`, so `AnimatePresence` fades it in/out with the run) and the cancel scrim +
 *  Stop pill. One component so `Editor.tsx` (already at its line budget) only has to render a
 *  single element for the whole feature. Nested inside `.e-stagetoast` (Task 11) so the Stop
 *  pill - the one piece here that isn't `position:fixed` - anchors to the stage's own bottom
 *  edge; the pointer and scrim are still `fixed` and unaffected by where this mounts. */
export function DirectorOverlay({ running, planning, model, pointerRef, progress, onCancel }: {
  running: boolean;
  planning: boolean;
  model?: string;
  pointerRef: RefObject<DirectorPointerHandle | null>;
  progress: { step: number; total: number } | null;
  onCancel: () => void;
}) {
  return (
    <>
      <AnimatePresence>{running && <DirectorPointer ref={pointerRef} />}</AnimatePresence>
      <DirectorScrim running={running} planning={planning} model={model} progress={progress} onCancel={onCancel} />
    </>
  );
}
