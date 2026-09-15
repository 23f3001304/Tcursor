import { motion } from "motion/react";
import { PLAY_SPRING, PRESS_TAP } from "./transportMotion";
import { setViewMode, useViewMode, VIEW_MODES } from "./viewMode";

export function ViewPicker() {
  const view = useViewMode();
  return (
    <div className="e-viewseg" role="group" aria-label="Preview view mode">
      {VIEW_MODES.map(({ id, label, title }) => (
        <motion.button
          key={id}
          type="button"
          className={view === id ? "on" : ""}
          title={title}
          aria-pressed={view === id}
          onClick={() => setViewMode(id)}
          whileTap={PRESS_TAP}
          transition={PLAY_SPRING}
        >
          {label}
        </motion.button>
      ))}
    </div>
  );
}
