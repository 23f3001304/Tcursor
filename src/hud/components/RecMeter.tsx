import { AnimatePresence, motion } from "motion/react";
import { VoiceWave } from "../../shared/wave/ui/VoiceWave";

export const METER_W = 150;
export const METER_H = 40;

const PULSE = { duration: 1.6, repeat: Infinity, ease: "easeInOut" as const };

const FROSTED = { opacity: 0, scale: 0.6, filter: "blur(6px)" };
const ARRIVE = {
  scale: { type: "spring" as const, stiffness: 420, damping: 15 },
  filter: { duration: 0.22 },
  opacity: { duration: 0.18 },
};

export function RecMeter({
  live,
  read,
  paused,
}: {
  live: boolean;
  read: () => { mic: number; sys: number };
  paused: boolean;
}) {
  const state = paused ? "paused" : "wave";
  return (
    <span className="take-slot">
      <AnimatePresence mode="wait" initial={false}>
        <motion.span
          key={state}
          className="take-slot-in"
          initial={FROSTED}
          animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
          exit={{ ...FROSTED, transition: { duration: 0.12 } }}
          transition={ARRIVE}
        >
          {state === "paused" ? (
            <motion.span className="take-paused" animate={{ opacity: [0.55, 1, 0.55] }} transition={PULSE}>
              Paused
            </motion.span>
          ) : (
            <VoiceWave w={METER_W} h={METER_H} read={read} live={live} />
          )}
        </motion.span>
      </AnimatePresence>
    </span>
  );
}
