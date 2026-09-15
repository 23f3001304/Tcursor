import type { ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";

const SHOWN = { opacity: 1, filter: "blur(0px)", scale: 1 };
const FROSTED = { opacity: 0, filter: "blur(14px)", scale: 0.92 };
const ARRIVE = {
  scale: { type: "spring" as const, stiffness: 340, damping: 15 },
  filter: { duration: 0.26, ease: [0.22, 1, 0.36, 1] as const },
  opacity: { duration: 0.2 },
};
const LEAVE = { duration: 0.14, ease: [0.4, 0, 1, 1] as const };

export const FROST = { shown: SHOWN, frosted: FROSTED, arrive: ARRIVE, leave: LEAVE };

export function StateSwap({
  take,
  onSettled,
  idle,
  pill,
}: {
  take: boolean;
  onSettled: (take: boolean) => void;
  idle: ReactNode;
  pill: ReactNode;
}) {
  return (
    <AnimatePresence mode="wait" initial={false} onExitComplete={() => onSettled(take)}>
      <motion.div
        key={take ? "take" : "idle"}
        className="swap"
        initial={FROSTED}
        animate={SHOWN}
        exit={{ ...FROSTED, transition: LEAVE }}
        transition={ARRIVE}
      >
        {take ? pill : idle}
      </motion.div>
    </AnimatePresence>
  );
}
