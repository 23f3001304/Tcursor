import { motion } from "motion/react";
import { useReducedMotion } from "../../lib/wave/ui/useReducedMotion";
import "../../lib/wave.css";

/** The timeline playhead: the brand mark turned 90 degrees - a vertical accent stroke with the
 *  dot at its head, instead of the triangular scrub handle it used to carry.
 *
 *  Three layers, all Motion-owned because all three are stateful:
 *  - the line's `left%`, tweened while paused and snapped while playing (a tween on a 60fps
 *    playback tick would lag the frame it is meant to mark);
 *  - the glow, crossfaded on `dragging` - never lit at rest, so the line stays a plain confident
 *    stroke until the user actually grabs it;
 *  - the drag ripple, a ring propagating out from the dot and decaying over ~300ms, mounted only
 *    while dragging. This is the one piece reduced motion drops: the playhead still moves, still
 *    glows, still reads as grabbed, it just does not pulse.
 *
 *  Extracted from `Timeline.tsx` (at its line cap) so Timeline renders one element for all of it.
 *  `.e-ph`/`.e-ph-glow` keep their timeline layout rules in `timeline.css`; the head and the ripple
 *  are wave-motif pieces and live in `wave.css` with the rest of the motif. */
export function Playhead({ pct, playing, dragging }: { pct: number; playing: boolean; dragging: boolean }) {
  const reduced = useReducedMotion();
  return (
    <motion.div className="e-ph" initial={false} animate={{ left: `${pct}%` }}
      transition={playing ? { duration: 0 } : { type: "tween", duration: 0.12, ease: "easeOut" }}>
      <motion.i className="e-ph-glow" initial={false} animate={{ opacity: dragging ? 1 : 0 }}
        transition={{ duration: 0.15, ease: "easeOut" }} />
      {dragging && !reduced && (
        <motion.i className="w-ph-ripple" initial={{ scale: 1, opacity: 0.55 }} animate={{ scale: 2.8, opacity: 0 }}
          transition={{ duration: 0.3, ease: "easeOut", repeat: Infinity }}
          style={{ transformOrigin: "center" }} />
      )}
      <motion.i className="w-ph-head" whileHover={{ scale: 1.15 }}
        transition={{ type: "spring", stiffness: 420, damping: 22 }} />
    </motion.div>
  );
}
