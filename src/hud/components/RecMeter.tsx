import { AnimatePresence, motion } from "motion/react";
import { VoiceWave } from "../../lib/wave/ui/VoiceWave";

/** Width and height of the meter's drawing area, px. `useHudWindowSize.ts`'s `SLOT_W` term
 *  carries this width, so the take pill's window stays sized to what the slot actually needs. */
export const METER_W = 150;
export const METER_H = 40;

const PULSE = { duration: 1.6, repeat: Infinity, ease: "easeInOut" as const };
/** Whichever of the two is arriving springs in from a frosted shrink; the leaving one melts
 *  out fast. The swap is the slot's one interstate and never changes its width. */
const FROSTED = { opacity: 0, scale: 0.6, filter: "blur(6px)" };
const ARRIVE = { scale: { type: "spring" as const, stiffness: 420, damping: 15 }, filter: { duration: 0.22 }, opacity: { duration: 0.18 } };

/** The take pill's level slot: one fixed-width box (`.take-slot`, `METER_W` wide) that shows
 *  one of two things, so the pill never changes width mid-take. `TakeBar` mounts it only while an
 *  audio source (mic or system) is on; with both off the slot is absent altogether.
 *
 *  - Paused: the word, breathing, where the wave was. No level arrives while paused
 *    (`Hud.tsx` subscribes `useAudioLevels` only while `recording && !paused`), so a wave here
 *    would be a flat line pretending to listen.
 *  - Otherwise: the live voice wave (`VoiceWave`), which drains to the line colour while `live`
 *    is false (a source enabled but no level report yet: permission pending, no device, a driver
 *    reset mid-take) rather than drawing a resting wave that would read as a working microphone.
 *
 *  State honesty (task-6 (c)/(i), user-reported): the levels themselves come from the Rust
 *  capture that is writing the WAV (`useAudioLevels`), so there is no path here that can show a
 *  level for audio this take is not recording. The meter's own geometry is not this file's
 *  business: it hands `VoiceWave` a box and a level getter, and `voiceWave.ts` decides what a
 *  frame looks like inside it. */
export function RecMeter({ live, read, paused }: {
  live: boolean; read: () => { mic: number; sys: number }; paused: boolean;
}) {
  const state = paused ? "paused" : "wave";
  return (
    <span className="take-slot">
      <AnimatePresence mode="wait" initial={false}>
        <motion.span key={state} className="take-slot-in" initial={FROSTED} animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
          exit={{ ...FROSTED, transition: { duration: 0.12 } }} transition={ARRIVE}>
          {state === "paused"
            ? <motion.span className="take-paused" animate={{ opacity: [0.55, 1, 0.55] }} transition={PULSE}>Paused</motion.span>
            : <VoiceWave w={METER_W} h={METER_H} read={read} live={live} />}
        </motion.span>
      </AnimatePresence>
    </span>
  );
}
