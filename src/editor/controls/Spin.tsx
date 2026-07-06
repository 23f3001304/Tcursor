import { motion } from "motion/react";
import { IconLoader2 } from "@tabler/icons-react";

/** A spinner driven by Motion (no CSS keyframes), used for preview/export loading. */
export function Spin({ size = 18 }: { size?: number }) {
  return (
    <motion.span style={{ display: "flex" }} animate={{ rotate: 360 }}
      transition={{ repeat: Infinity, duration: 0.8, ease: "linear" }}>
      <IconLoader2 size={size} />
    </motion.span>
  );
}
