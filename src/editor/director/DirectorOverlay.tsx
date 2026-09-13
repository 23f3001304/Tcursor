import type { RefObject } from "react";
import { AnimatePresence, motion } from "motion/react";
import { SweepWave } from "../../lib/wave/ui/SweepWave";
import { DirectorPointer, type DirectorPointerHandle } from "./DirectorPointer";
import { DirectorScrim } from "./DirectorScrim";

/** Everything the AI director's choreographed reveal renders: the fake pointer (mounted only
 *  while `running`, so `AnimatePresence` fades it in/out with the run), the pass's sweeping wave,
 *  and the cancel scrim + Stop pill. One component so `Editor.tsx` (already at its line budget)
 *  only has to render a single element for the whole feature. Nested inside `.e-stagetoast`
 *  (Task 11) so the wave and the Stop pill - the pieces here that aren't `position:fixed` -
 *  anchor to the stage's own bottom edge; the pointer and scrim are still `fixed` and unaffected
 *  by where this mounts.
 *
 *  The wave is the pass's progress readout, replacing the indeterminate pulsing dot the Stop pill
 *  used to carry (benchmark (c) item 7: a generic spinner at the one moment the brand should feel
 *  most in control). While planning it sweeps on its own clock - there is no step count yet - and
 *  switches to a determinate head at `step/total` the moment the reveal starts. */
export function DirectorOverlay({ running, planning, model, pointerRef, progress, onCancel }: {
  running: boolean;
  planning: boolean;
  model?: string;
  pointerRef: RefObject<DirectorPointerHandle | null>;
  progress: { step: number; total: number } | null;
  onCancel: () => void;
}) {
  const pct = !planning && progress && progress.total > 0
    ? (progress.step / progress.total) * 100
    : undefined;
  return (
    <>
      <AnimatePresence>{running && <DirectorPointer ref={pointerRef} />}</AnimatePresence>
      <AnimatePresence>
        {running && (
          <motion.div className="w-director" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 6 }} transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
            <SweepWave w={260} h={30} pct={pct} tone="ai" />
          </motion.div>
        )}
      </AnimatePresence>
      <DirectorScrim running={running} planning={planning} model={model} progress={progress} onCancel={onCancel} />
    </>
  );
}
