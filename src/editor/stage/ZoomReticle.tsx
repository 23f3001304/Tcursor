import { motion } from "motion/react";

const SPRING = { type: "spring", stiffness: 420, damping: 34, mass: 0.6 } as const;

export function ZoomReticle({
  x,
  y,
  aiming,
  dragging,
  onPointerDown,
}: {
  x: number;
  y: number;
  aiming: boolean;
  dragging: boolean;
  onPointerDown?: (e: React.PointerEvent) => void;
}) {
  return (
    <motion.div
      className={`e-zreticle${aiming ? " aim" : ""}${dragging ? " drag" : ""}`}
      initial={false}
      animate={{ left: `${x * 100}%`, top: `${y * 100}%` }}
      transition={dragging ? { duration: 0 } : SPRING}
      title={aiming ? "Drag to aim this zoom" : "This zoom's aim point"}
      onPointerDown={onPointerDown}
    >
      <i />
    </motion.div>
  );
}
