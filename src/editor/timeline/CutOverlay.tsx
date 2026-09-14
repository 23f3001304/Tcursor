import { memo } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import type { Cut } from "../../lib/edit";

/** The stretches a cut removes, hatched across the whole track stack - the trim overlay's idiom one
 *  plane down, and for the same reason: what is gone is gone from every lane at once, not from the
 *  Time lane alone. The hatch itself is a pattern fill (`.e-cut`, timeline.css), no border; the 1px
 *  accent edges only appear on hover, so a resting timeline stays quiet.
 *
 *  Click selects the cut (the id goes into the editor's one selection), which opens `CutInspector`
 *  and arms Delete through the shared delete-selection path in `useEditorKeymap`. Nothing here
 *  drags: a cut's edges are edited numerically in the inspector, where a millisecond is reachable. */
export const CutOverlay = memo(function CutOverlay({ cuts, dur, sel, onSel }: {
  cuts: Cut[]; dur: number; sel: string | null; onSel: (id: string | null) => void;
}) {
  const still = useReducedMotion();
  if (dur <= 0) return null;
  return (
    <AnimatePresence initial={false}>
      {cuts.map((c) => {
        const from = Math.max(0, Math.min(c.start_ms, dur));
        const to = Math.max(from, Math.min(c.end_ms, dur));
        return (
          <motion.div key={c.id} data-cut-id={c.id} className={`e-cut${sel === c.id ? " sel" : ""}`}
            style={{ left: `${(from / dur) * 100}%`, width: `${((to - from) / dur) * 100}%` }}
            title={`Cut: ${((c.end_ms - c.start_ms) / 1000).toFixed(1)} s. Click to select, Delete to remove`}
            initial={still ? false : { opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
            transition={{ duration: still ? 0 : 0.14 }}
            onPointerDown={(e) => { e.stopPropagation(); onSel(c.id); }} />
        );
      })}
    </AnimatePresence>
  );
});
