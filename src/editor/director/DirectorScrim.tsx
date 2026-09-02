import { useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";
import { engineDisplayName } from "./engineName";

/** Cancel surface for a running AI-director pass: a transparent, full-viewport scrim (catches ANY
 *  pointerdown) plus an Escape listener, both requesting cancel - and a small "Stop" pill
 *  (bottom-center, `Toast`-styled) restating live progress, which also cancels on click. Mounted
 *  only while `running`; cancelling never aborts the step already in flight (see
 *  `useDirector.ts`'s `reveal` loop) - this only stops the NEXT one from starting. While
 *  `planning` (the `ai_plan` fetch itself - can take minutes on a first-run model load, `useDirector`'s
 *  `planOrCancel`), the pill instead reads "Asking <model>…" with a pulsing (indeterminate -
 *  there's no step count yet) dot, and Esc/click here cancels the PASS, not just a future step
 *  (`useDirector.run` checks `cancelRef` the moment the fetch resolves and bails before applying
 *  anything). */
export function DirectorScrim({ running, planning, model, progress, onCancel }: {
  running: boolean;
  planning: boolean;
  model?: string;
  progress: { step: number; total: number } | null;
  onCancel: () => void;
}) {
  useEffect(() => {
    if (!running) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onCancel(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [running, onCancel]);

  // Short derived name in the copy (ux audit #17 printed the full raw id) - the full id is still
  // one hover away via the button's own `title`.
  const shortModel = model ? engineDisplayName(model) : "";
  const label = planning
    ? `Asking ${shortModel || "the model"}… first runs can take a while — Esc to stop`
    : progress ? `Directing · step ${progress.step}/${progress.total} — Esc to stop` : "Directing… — Esc to stop";

  return (
    <>
      {running && <div className="e-director-scrim" onPointerDown={onCancel} />}
      <AnimatePresence>
        {running && (
          <motion.div className="e-director-stop" title={model} onPointerDown={(e) => { e.stopPropagation(); onCancel(); }}
            initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 8 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
            {planning && (
              // Indeterminate progress: no step count exists yet, so pulse rather than fill.
              <motion.span aria-hidden style={{ display: "inline-block", width: 6, height: 6, borderRadius: 999, background: "currentColor", marginRight: 8 }}
                animate={{ opacity: [0.35, 1, 0.35] }} transition={{ duration: 1.1, repeat: Infinity, ease: "easeInOut" }} />
            )}
            {label}
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
