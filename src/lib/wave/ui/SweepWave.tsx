import { useCallback, useEffect, useRef } from "react";
import { motion } from "motion/react";
import { headFor, progressAmp, sweepEnvelope } from "../math/sweep";
import { sinePath } from "../math/sine";
import { useReducedMotion } from "./useReducedMotion";
import "../../wave.css";

const LAMBDA = 58;      // spatial period, px - about four crests across a dialog-width sweep
const TRAVEL_S = 2.4;   // seconds for the waveform itself to slide one period under the envelope
const DOT_R = 3.5;

/** The processing wave: one sine sweeping left to right with the dot as its scanning head.
 *
 *  Two modes, one component. With no `pct` (the AI director, which knows steps but not work left)
 *  the head is swept from the clock. With a `pct` the head IS the percentage - it sits exactly at
 *  `pct` of the width on the wave's trailing crest, and the whole wave's amplitude decays to flat
 *  as that reaches 100, so "nearly done" is legible from the shape alone before the number is read.
 *
 *  `done` flattens the wave, fades the flat line away and hands over to the checkmark, which
 *  draws itself from the dot's last position - the one stateful transition here, so Motion owns
 *  it while the sweep itself stays a rAF loop writing path data. The line has to go: a flat
 *  full-width accent stroke under a tick read as a red bar with a check drawn over it (owner,
 *  2026-09-14), not as a wave that had settled.
 *
 *  Under reduced motion the wave is drawn once at the current `pct` with no travel and no sweep
 *  loop; a determinate export still shows real progress, an indeterminate pass shows a still wave. */
export function SweepWave({ w = 300, h = 34, pct, done = false, tone }: {
  w?: number; h?: number; pct?: number; done?: boolean; tone?: "ai";
}) {
  const path = useRef<SVGPathElement>(null);
  const dot = useRef<SVGCircleElement>(null);
  const reduced = useReducedMotion();
  const mid = h / 2;
  const amp = (h / 2 - DOT_R - 2) * progressAmp(pct ?? 0);

  // Read per frame, never a dependency: an export's `pct` changes many times a second, and if the
  // loop below restarted on each one the waveform's travel would reset its phase every percent.
  const live = useRef({ pct, amp, done, w, mid, reduced });
  live.current = { pct, amp, done, w, mid, reduced };

  const draw = useCallback((t: number) => {
    const { pct: p, amp: a0, done: fin, w: width, mid: m, reduced: red } = live.current;
    // Done parks the head mid-span rather than off the right edge: the dot has to be somewhere
    // the checkmark can legibly draw itself, and the wave under it is flat by then anyway.
    const head = fin ? 0.5 : headFor(t, p);
    const a = fin ? 0 : a0;
    const phase = red ? 0 : (-2 * Math.PI * t) / TRAVEL_S;
    path.current?.setAttribute("d", sinePath({
      w: width, mid: m, amp: a, lambda: LAMBDA, phase, envelope: (x) => sweepEnvelope(x, head),
    }));
    dot.current?.setAttribute("cx", (head * width).toFixed(2));
    // The dot rides the wave it is scanning, so head and crest are never two separate marks.
    dot.current?.setAttribute("cy", (m - a * Math.sin((2 * Math.PI * head * width) / LAMBDA + phase)).toFixed(2));
  }, []);

  // Still frames: whenever the loop is NOT running, any prop change has to redraw by itself.
  useEffect(() => { if (reduced || done) draw(0); }, [draw, reduced, done, pct, amp, w, h, mid]);

  useEffect(() => {
    if (reduced || done) return;
    let raf = 0;
    const start = performance.now();
    const tick = (now: number) => { raf = requestAnimationFrame(tick); draw((now - start) / 1000); };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [draw, reduced, done]);

  return (
    <svg className={`w-wave w-sweep${tone === "ai" ? " ai" : ""}`} width={w} height={h}
      viewBox={`0 0 ${w} ${h}`} aria-hidden="true" focusable="false">
      <motion.path ref={path} className="w-stroke front" animate={{ opacity: done ? 0 : 1 }}
        transition={reduced ? { duration: 0 } : { duration: 0.28, ease: [0.4, 0, 0.2, 1] }} />
      <motion.circle ref={dot} className="w-dot" cx={0} cy={mid} r={DOT_R}
        animate={{ scale: done ? 0 : 1 }}
        transition={reduced ? { duration: 0 } : { type: "spring", stiffness: 420, damping: 26 }}
        style={{ transformBox: "fill-box", transformOrigin: "center" }} />
      {done && (
        <motion.path className="w-check" d={`M ${w / 2 - 8} ${mid} l 5.5 5.5 L ${w / 2 + 9} ${mid - 7}`}
          initial={reduced ? false : { pathLength: 0, opacity: 0 }} animate={{ pathLength: 1, opacity: 1 }}
          transition={reduced ? { duration: 0 } : { duration: 0.32, ease: [0.4, 0, 0.2, 1] }} />
      )}
    </svg>
  );
}
