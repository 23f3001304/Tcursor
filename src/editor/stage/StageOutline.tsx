import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import type { CamSample, PreviewLayout } from "../../shared/ipc";
import { camAt } from "./camera/camera";
import { outlineRect } from "../director/review/outline";

const FADE = { duration: 0.14, ease: [0.4, 0, 0.2, 1] as const };

export function StageOutline({
  rect,
  canvasW,
  canvasH,
  layout,
  track,
  tOut,
}: {
  rect: [number, number, number, number] | null;
  canvasW: number;
  canvasH: number;
  layout: PreviewLayout | null;
  track: CamSample[];
  tOut: number;
}) {
  const still = useReducedMotion();
  const box = rect ? outlineRect(rect, canvasW, canvasH, layout, camAt(track, tOut)) : null;
  const pct = (v: number) => `${v * 100}%`;
  return (
    <AnimatePresence>
      {box && (
        <motion.div
          key="ai-outline"
          className="e-outline"
          aria-hidden
          style={{ left: pct(box.left), top: pct(box.top), width: pct(box.width), height: pct(box.height) }}
          initial={still ? false : { opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={still ? { duration: 0 } : FADE}
        />
      )}
    </AnimatePresence>
  );
}
