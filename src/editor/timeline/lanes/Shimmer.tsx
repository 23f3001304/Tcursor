import { motion } from "motion/react";

export function Shimmer({ className }: { className?: string }) {
  return (
    <div className={`e-shimmer ${className ?? ""}`}>
      <motion.div
        className="e-shimmer-sweep"
        animate={{ x: ["-100%", "350%"] }}
        transition={{ duration: 1.6, repeat: Infinity, ease: "linear" }}
      />
    </div>
  );
}
