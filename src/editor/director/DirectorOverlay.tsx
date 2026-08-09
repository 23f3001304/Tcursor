import type { RefObject } from "react";
import { AnimatePresence } from "motion/react";
import { DirectorPointer, type DirectorPointerHandle } from "./DirectorPointer";
import { DirectorScrim } from "./DirectorScrim";

/** Everything the AI director's choreographed reveal renders at the editor root: the fake pointer
 *  (mounted only while `running`, so `AnimatePresence` fades it in/out with the run) and the
 *  cancel scrim + Stop pill. One component so `Editor.tsx` (already at its line budget) only has
 *  to render a single element for the whole feature. */
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
