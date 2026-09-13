import { motion } from "motion/react";
import { flowSeconds, dotPulses, dotTint, WAVE_LAMBDA, type MarkState } from "./brandWave";
import { useReducedMotion } from "./wave/ui/useReducedMotion";

const WAVE_D = "M -14.0 50.0 L -11.0 50.5 L -8.0 51.8 L -5.0 53.8 L -2.0 56.5 L 1.0 59.7 L 4.0 63.3 L 7.0 67.1 L 10.0 70.9 L 13.0 74.6 L 16.0 78.1 L 19.0 81.0 L 22.0 83.3 L 25.0 85.0 L 28.0 85.9 L 31.0 85.9 L 34.0 85.2 L 37.0 83.6 L 40.0 81.4 L 43.0 78.5 L 46.0 75.1 L 49.0 71.5 L 52.0 67.6 L 55.0 63.8 L 58.0 60.2 L 61.0 56.9 L 64.0 54.1 L 67.0 52.0 L 70.0 50.6 L 73.0 50.0 L 76.0 50.2 L 79.0 51.3 L 82.0 53.1 L 85.0 55.5 L 88.0 58.6 L 91.0 62.1 L 94.0 65.8 L 97.0 69.7 L 100.0 73.4 L 103.0 77.0 L 106.0 80.1 L 109.0 82.6 L 112.0 84.5 L 115.0 85.7 L 118.0 86.0 L 121.0 85.5 L 124.0 84.2 L 127.0 82.2 L 130.0 79.5 L 133.0 76.3 L 136.0 72.7 L 139.0 68.9 L 142.0 65.1";

// Tiled copies of the wave, spaced one WAVE_LAMBDA apart, only rendered while actually flowing.
// Wide enough that some copy covers every point of the viewBox across the WHOLE animated x range
// (not just its two endpoints) - see brandWave.ts's WAVE_LAMBDA doc for why one copy either side
// isn't enough on its own.
const WAVE_COPIES = [-WAVE_LAMBDA * 2, -WAVE_LAMBDA, 0, WAVE_LAMBDA, WAVE_LAMBDA * 2];

/** The TCursor wave mark. `state` (default `"idle"` - today's plain static rendering, byte-for-
 *  byte unchanged for every caller that doesn't pass it) drives the Task 39 "living brand" touches:
 *  the wave flows in a seamless loop (a linear, repeating horizontal translate by one period,
 *  `WAVE_LAMBDA`) while recording/exporting/directing, and the REC dot pulses (a repeating Motion
 *  spring on scale+opacity) while recording. Both are transform/opacity-only - cheap - and gated
 *  off entirely at `prefers-reduced-motion`. Callers additionally gate `state` itself by the
 *  `ui.animated_brand` setting before passing it in - a plain feel-knob check that belongs at the
 *  call site, not inside this shared, dependency-free component. */
export function TcursorMark({ size = 20, dotColor = "var(--e-primary, #ef4444)", state = "idle", pct }: {
  size?: number; dotColor?: string; state?: MarkState; pct?: number;
}) {
  const reduced = useReducedMotion();
  const seconds = reduced ? 0 : flowSeconds(state, pct);
  const pulsing = !reduced && dotPulses(state);
  const fill = dotTint(state) ?? dotColor;

  return (
    <svg width={size} height={size} viewBox="-33 22 194 83" aria-hidden="true" focusable="false">
      {seconds > 0 ? (
        <motion.g animate={{ x: [0, -WAVE_LAMBDA] }} transition={{ x: { duration: seconds, repeat: Infinity, ease: "linear" } }}>
          {WAVE_COPIES.map((dx) => (
            <path key={dx} d={WAVE_D} fill="none" stroke="currentColor" strokeWidth={30} strokeLinecap="butt"
              transform={`translate(${dx},0)`} />
          ))}
        </motion.g>
      ) : (
        <path d={WAVE_D} fill="none" stroke="currentColor" strokeWidth={30} strokeLinecap="butt" />
      )}
      <motion.circle cx={103} cy={33} r={7} fill={fill}
        animate={pulsing ? { scale: 1.2, opacity: 0.7 } : { scale: 1, opacity: 1 }}
        transition={pulsing
          ? { type: "spring", stiffness: 300, damping: 10, repeat: Infinity, repeatType: "reverse" }
          : { duration: 0 }}
        style={{ transformBox: "fill-box", transformOrigin: "center" }} />
    </svg>
  );
}
