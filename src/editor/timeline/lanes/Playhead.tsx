import { motion } from "motion/react";
import { useReducedMotion } from "../../../shared/wave/ui/useReducedMotion";
import "../../../shared/wave.css";

export function Playhead({ pct, playing, dragging }: { pct: number; playing: boolean; dragging: boolean }) {
  const reduced = useReducedMotion();
  return (
    <motion.div
      className="e-ph"
      initial={false}
      animate={{ left: `${pct}%` }}
      transition={playing ? { duration: 0 } : { type: "tween", duration: 0.12, ease: "easeOut" }}
    >
      <motion.i
        className="e-ph-glow"
        initial={false}
        animate={{ opacity: dragging ? 1 : 0 }}
        transition={{ duration: 0.15, ease: "easeOut" }}
      />
      {dragging && !reduced && (
        <motion.i
          className="w-ph-ripple"
          initial={{ scale: 1, opacity: 0.55 }}
          animate={{ scale: 2.8, opacity: 0 }}
          transition={{ duration: 0.3, ease: "easeOut", repeat: Infinity }}
          style={{ transformOrigin: "center" }}
        />
      )}
      <motion.i
        className="w-ph-head"
        whileHover={{ scale: 1.15 }}
        transition={{ type: "spring", stiffness: 420, damping: 22 }}
      />
    </motion.div>
  );
}
