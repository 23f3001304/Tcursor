import { motion } from "motion/react";

/** A quiet "still loading" skeleton for the timeline (Filmstrip/AudioTrack): the same `--e-card`/
 *  `--e-border` box the loaded content sits in (via `className`, so height/radius match exactly
 *  and nothing jumps once real content replaces it), with a slow highlight sweep. Per the
 *  fake-polish philosophy (every feel knob is an intentional, designed motion, not a generic
 *  browser-default loader) and the Global Constraints, the sweep is a Motion-animated gradient
 *  layer (`x`, a repeating tween) - NOT a CSS `@keyframes` loop. */
export function Shimmer({ className }: { className?: string }) {
  return (
    <div className={`e-shimmer ${className ?? ""}`}>
      <motion.div className="e-shimmer-sweep"
        animate={{ x: ["-100%", "350%"] }}
        transition={{ duration: 1.6, repeat: Infinity, ease: "linear" }} />
    </div>
  );
}
