import type { RefObject } from "react";
import { AnimatePresence, motion } from "motion/react";
import { SweepWave } from "../../shared/wave/ui/SweepWave";
import { DirectorPointer, type DirectorPointerHandle } from "./DirectorPointer";
import { DirectorScrim } from "./DirectorScrim";

export function DirectorOverlay({
  running,
  planning,
  model,
  pointerRef,
  progress,
  onCancel,
}: {
  running: boolean;
  planning: boolean;
  model?: string;
  pointerRef: RefObject<DirectorPointerHandle | null>;
  progress: { step: number; total: number } | null;
  onCancel: () => void;
}) {
  const pct =
    !planning && progress && progress.total > 0 ? (progress.step / progress.total) * 100 : undefined;
  return (
    <>
      <AnimatePresence>{running && <DirectorPointer ref={pointerRef} />}</AnimatePresence>
      <AnimatePresence>
        {running && (
          <motion.div
            className="w-director"
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 6 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
          >
            <SweepWave w={260} h={30} pct={pct} tone="ai" />
          </motion.div>
        )}
      </AnimatePresence>
      <DirectorScrim
        running={running}
        planning={planning}
        model={model}
        progress={progress}
        onCancel={onCancel}
      />
    </>
  );
}
