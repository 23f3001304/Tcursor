import type { ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";

// The interstate: whichever of the two is leaving frosts over and shrinks (fast, 140ms), then
// the other one arrives from the same frost with a springy scale that overshoots a hair - the
// "gooey" landing the owner asked for (2026-09-14), on the scale only; blur and opacity are plain
// tweens, since a spring on a blur reads as flicker rather than bounce.
const SHOWN = { opacity: 1, filter: "blur(0px)", scale: 1 };
const FROSTED = { opacity: 0, filter: "blur(14px)", scale: 0.92 };
const ARRIVE = {
  scale: { type: "spring" as const, stiffness: 340, damping: 15 },
  filter: { duration: 0.26, ease: [0.22, 1, 0.36, 1] as const },
  opacity: { duration: 0.2 },
};
const LEAVE = { duration: 0.14, ease: [0.4, 0, 1, 1] as const };

/** The interstate's own constants, so a surface that swaps in elsewhere under the same HUD (the
 *  take pill's Sources sheet) frosts with exactly these numbers rather than a second, drifting
 *  copy of them. */
export const FROST = { shown: SHOWN, frosted: FROSTED, arrive: ARRIVE, leave: LEAVE };

/** The HUD's two bar states, swapped with a frost-and-spring interstate. `mode="wait"`: the
 *  leaving state finishes frosting out before the arriving one mounts, and `onSettled` fires in
 *  that gap with the state that is about to show - `Hud` uses it to start the window glide
 *  (`useHudWindowSize`) at the same moment the pill (or the bar) starts arriving, so the two
 *  animations land together. `initial={false}`: no interstate on first mount. */
export function StateSwap({ take, onSettled, idle, pill }: {
  take: boolean; onSettled: (take: boolean) => void; idle: ReactNode; pill: ReactNode;
}) {
  return (
    <AnimatePresence mode="wait" initial={false} onExitComplete={() => onSettled(take)}>
      <motion.div key={take ? "take" : "idle"} className="swap" initial={FROSTED} animate={SHOWN}
        exit={{ ...FROSTED, transition: LEAVE }} transition={ARRIVE}>
        {take ? pill : idle}
      </motion.div>
    </AnimatePresence>
  );
}
