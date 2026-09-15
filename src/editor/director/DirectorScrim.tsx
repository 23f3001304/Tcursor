import { useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";
import { engineDisplayName } from "./engineName";

export function DirectorScrim({
  running,
  planning,
  model,
  progress,
  onCancel,
}: {
  running: boolean;
  planning: boolean;
  model?: string;
  progress: { step: number; total: number } | null;
  onCancel: () => void;
}) {
  useEffect(() => {
    if (!running) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [running, onCancel]);

  const shortModel = model ? engineDisplayName(model) : "";
  const label = planning
    ? `Thinking with ${shortModel || "the model"}… first runs can take a while - Esc to stop`
    : progress
      ? `Replaying · ${progress.step}/${progress.total} - Esc to stop`
      : "Applying…";

  return (
    <>
      {running && <div className="e-director-scrim" onPointerDown={onCancel} />}
      <AnimatePresence>
        {running && (
          <motion.div
            className="e-director-stop"
            title={model}
            onPointerDown={(e) => {
              e.stopPropagation();
              onCancel();
            }}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 8 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
          >
            {label}
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
