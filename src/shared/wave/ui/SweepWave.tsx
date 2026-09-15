import { useCallback, useEffect, useRef } from "react";
import { motion } from "motion/react";
import { headFor, progressAmp, sweepEnvelope } from "../math/sweep";
import { sinePath } from "../math/sine";
import { useReducedMotion } from "./useReducedMotion";
import "../../wave.css";

const LAMBDA = 58;
const TRAVEL_S = 2.4;
const DOT_R = 3.5;

export function SweepWave({
  w = 300,
  h = 34,
  pct,
  done = false,
  tone,
}: {
  w?: number;
  h?: number;
  pct?: number;
  done?: boolean;
  tone?: "ai";
}) {
  const path = useRef<SVGPathElement>(null);
  const dot = useRef<SVGCircleElement>(null);
  const reduced = useReducedMotion();
  const mid = h / 2;
  const amp = (h / 2 - DOT_R - 2) * progressAmp(pct ?? 0);

  const live = useRef({ pct, amp, done, w, mid, reduced });
  live.current = { pct, amp, done, w, mid, reduced };

  const draw = useCallback((t: number) => {
    const { pct: p, amp: a0, done: fin, w: width, mid: m, reduced: red } = live.current;
    const head = fin ? 0.5 : headFor(t, p);
    const a = fin ? 0 : a0;
    const phase = red ? 0 : (-2 * Math.PI * t) / TRAVEL_S;
    path.current?.setAttribute(
      "d",
      sinePath({
        w: width,
        mid: m,
        amp: a,
        lambda: LAMBDA,
        phase,
        envelope: (x) => sweepEnvelope(x, head),
      }),
    );
    dot.current?.setAttribute("cx", (head * width).toFixed(2));
    dot.current?.setAttribute(
      "cy",
      (m - a * Math.sin((2 * Math.PI * head * width) / LAMBDA + phase)).toFixed(2),
    );
  }, []);

  useEffect(() => {
    if (reduced || done) draw(0);
  }, [draw, reduced, done, pct, amp, w, h, mid]);

  useEffect(() => {
    if (reduced || done) return;
    let raf = 0;
    const start = performance.now();
    const tick = (now: number) => {
      raf = requestAnimationFrame(tick);
      draw((now - start) / 1000);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [draw, reduced, done]);

  return (
    <svg
      className={`w-wave w-sweep${tone === "ai" ? " ai" : ""}`}
      width={w}
      height={h}
      viewBox={`0 0 ${w} ${h}`}
      aria-hidden="true"
      focusable="false"
    >
      <motion.path
        ref={path}
        className="w-stroke front"
        animate={{ opacity: done ? 0 : 1 }}
        transition={reduced ? { duration: 0 } : { duration: 0.28, ease: [0.4, 0, 0.2, 1] }}
      />
      <motion.circle
        ref={dot}
        className="w-dot"
        cx={0}
        cy={mid}
        r={DOT_R}
        animate={{ scale: done ? 0 : 1 }}
        transition={reduced ? { duration: 0 } : { type: "spring", stiffness: 420, damping: 26 }}
        style={{ transformBox: "fill-box", transformOrigin: "center" }}
      />
      {done && (
        <motion.path
          className="w-check"
          d={`M ${w / 2 - 8} ${mid} l 5.5 5.5 L ${w / 2 + 9} ${mid - 7}`}
          initial={reduced ? false : { pathLength: 0, opacity: 0 }}
          animate={{ pathLength: 1, opacity: 1 }}
          transition={reduced ? { duration: 0 } : { duration: 0.32, ease: [0.4, 0, 0.2, 1] }}
        />
      )}
    </svg>
  );
}
