import { useEffect, useRef, type RefObject } from "react";
import { motion } from "motion/react";
import { BREATH_MAX, BREATH_PERIOD_S } from "../math/voiceWave";
import { sinePath, sineY, TAU } from "../math/sine";
import { useReducedMotion } from "./useReducedMotion";
import "../../wave.css";

/** Seconds for one slow drift of the small idle wave, and of the wide horizon wave (benchmark
 *  (d): the horizon's period is ~4s, slow enough to read as a held breath rather than as motion). */
const IDLE_DRIFT_S = 2.6;
const HORIZON_PERIOD_S = 4;

interface Drift { w: number; mid: number; amp: number; lambda: number }

/** The drift loop both quiet waves share: writes the sine's `d` and parks the dot ON the wave at
 *  `dotX` every frame - or draws the pair once and stops, under reduced motion. The dot's y is
 *  read back from the same `sineY` that drew the stroke, so it can never float off the line. */
function useDrift(path: RefObject<SVGPathElement | null>, dot: RefObject<SVGCircleElement | null>,
  dotX: number, s: Drift, seconds: number, reduced: boolean) {
  const { w, mid, amp, lambda } = s;
  useEffect(() => {
    const draw = (t: number) => {
      const spec = { w, mid, amp, lambda, phase: (-TAU * t) / seconds };
      path.current?.setAttribute("d", sinePath(spec));
      dot.current?.setAttribute("cy", sineY(spec, dotX).toFixed(2));
    };
    if (reduced) { draw(0); return; }
    let raf = 0;
    const start = performance.now();
    const tick = (now: number) => { raf = requestAnimationFrame(tick); draw((now - start) / 1000); };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [path, dot, dotX, w, mid, amp, lambda, seconds, reduced]);
}

/** Motion config for the breathing dot - the mark's idle tell, shared by both waves below. */
const BREATHE = {
  animate: { scale: [1, BREATH_MAX, 1] },
  transition: { duration: BREATH_PERIOD_S, repeat: Infinity, ease: "easeInOut" as const },
  style: { transformBox: "fill-box" as const, transformOrigin: "center" },
};

/** The app's generic in-progress indicator: a small low-amplitude wave with the breathing dot.
 *  Replaces the rotating loader everywhere `Spin` is used, so the brand mark - not a generic ring -
 *  is what the app shows while it is working. The waveform drifts slowly so it still reads as
 *  "running" rather than frozen; the dot's breathe is the same 2s cycle the idle meter uses. */
export function IdleWave({ size = 18 }: { size?: number }) {
  const path = useRef<SVGPathElement>(null);
  const dot = useRef<SVGCircleElement>(null);
  const reduced = useReducedMotion();
  const w = Math.round(size * 1.7), h = size, dotX = w - size * 0.18;
  useDrift(path, dot, dotX, { w, mid: h / 2, amp: h * 0.2, lambda: w * 0.62 }, IDLE_DRIFT_S, reduced);
  return (
    <svg className="w-wave w-idle" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true" focusable="false">
      <path ref={path} className="w-stroke front" strokeWidth={2} />
      <motion.circle ref={dot} className="w-dot" cx={dotX} cy={h / 2} r={size * 0.15} {...(reduced ? {} : BREATHE)} />
    </svg>
  );
}

/** The empty and first-run state: a very-low-amplitude horizon wave in the wave motif's blue,
 *  with the dot flying in from off the left edge and landing on it - a ~600ms arrival that
 *  overshoots its mark by 8% before settling, so a blank stage has a beginning instead of just
 *  being blank. Under reduced motion the dot is simply already there. */
export function HorizonWave({ w = 220, h = 34 }: { w?: number; h?: number }) {
  const path = useRef<SVGPathElement>(null);
  const dot = useRef<SVGCircleElement>(null);
  const reduced = useReducedMotion();
  const landing = w * 0.62; // the dot lands off-centre, on the golden section of the horizon
  const travel = landing + 24;
  useDrift(path, dot, landing, { w, mid: h / 2, amp: h * 0.16, lambda: w * 0.8 }, HORIZON_PERIOD_S, reduced);
  return (
    <svg className="w-wave w-horizon" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true" focusable="false">
      <path ref={path} className="w-stroke front" />
      <motion.circle ref={dot} className="w-dot" cx={landing} cy={h / 2} r={3.5}
        initial={reduced ? false : { x: -travel, opacity: 0 }}
        animate={{ x: [-travel, travel * 0.08, 0], opacity: [0, 1, 1] }}
        transition={reduced ? { duration: 0 } : { duration: 0.6, times: [0, 0.72, 1], ease: [0.22, 1, 0.36, 1] }} />
    </svg>
  );
}
