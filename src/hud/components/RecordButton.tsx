import { motion } from "motion/react";

/** The idle card's hero: the one red button across its bottom, "Record". What is left of
 *  `RecordingControls` now that the take itself has its own pill (`TakeBar`) - the timer, the
 *  Paused state and the Pause/Stop pair live there. `label` lets the card show an export's
 *  progress in the button's own place while it is disabled. Its press is a Motion spring rather
 *  than `hud.css`'s `.btn:active` scale (that rule excludes `.rec` so the two never fight over
 *  one `transform`). */
export function RecordButton({ disabled, onClick, label = "Record" }: { disabled: boolean; onClick: () => void; label?: string }) {
  return (
    <motion.button className="btn rec" onClick={onClick} disabled={disabled}
      whileTap={{ scale: 0.97 }} transition={{ type: "spring", stiffness: 500, damping: 30 }}>
      <span className="dot" />{label}
    </motion.button>
  );
}
