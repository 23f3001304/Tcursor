import { motion } from "motion/react";
import { PLAY_SPRING, PRESS_TAP } from "./transportMotion";
import { setViewMode, useViewMode, VIEW_MODES } from "./viewMode";

/** The transport's view-mode control: Fit / Fill / 100%, sitting in the right group beside the
 *  quality chip rather than floating over the preview. Takes no props - the choice is session UI
 *  state shared with `Stage` through `viewMode.ts` (see the note there). */
export function ViewPicker() {
  const view = useViewMode();
  return (
    <div className="e-viewseg" role="group" aria-label="Preview view mode">
      {VIEW_MODES.map(({ id, label, title }) => (
        <motion.button key={id} type="button" className={view === id ? "on" : ""} title={title}
          aria-pressed={view === id} onClick={() => setViewMode(id)}
          whileTap={PRESS_TAP} transition={PLAY_SPRING}>
          {label}
        </motion.button>
      ))}
    </div>
  );
}
