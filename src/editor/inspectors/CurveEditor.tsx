import { useEffect, useRef, useState } from "react";
import { motion, useReducedMotion } from "motion/react";
import { formatCubic, parseCubic } from "../../lib/cubicBezier";
import { springOf } from "../../lib/spring";
import { CURVE_GLYPHS } from "../timeline/curveGlyphs";
import { EasingPicker } from "./EasingPicker";
import { SpringControls } from "./SpringControls";
import {
  clientToCurve, curveOf, curvePath, handlePct, nudgeHandle, setHandle, springPathOf, VIEW_BOX, type Cubic,
} from "./curveMath";

const NUDGE: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1],
};
const HANDLE_SPRING = { type: "spring" as const, stiffness: 420, damping: 34 };

/** The transition curve, as one compact block: the named-curve row, then ONE canvas drawing the
 *  selected curve. This is the drag-the-dots editor the owner asked back for, rebuilt without what
 *  they vetoed - the six-card grid and the card that expanded in place. A cubic's two control
 *  points are draggable dots over the canvas (dragging turns the row to Custom, the same math as
 *  before); a spring is not a cubic, so it draws its sampled oscillator and is tuned by the two
 *  sliders underneath instead. */
export function CurveEditor({ value, onChange, label = "Transition curve" }: {
  value: string; onChange: (easing: string) => void; label?: string;
}) {
  const still = useReducedMotion();
  const spr = springOf(value);
  // The in-flight drag, so the curve follows the pointer without one op per pointermove. Cleared
  // when the committed `value` comes back from the backend (or at once, if it is unchanged).
  const [draft, setDraft] = useState<Cubic | null>(null);
  const [grab, setGrab] = useState<0 | 1 | null>(null);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => { setDraft(null); }, [value]);

  const cur = draft ?? curveOf(value);
  const commit = (c: Cubic) => onChange(formatCubic(c[0], c[1], c[2], c[3]));
  // A named curve draws its own published glyph path (the one the timeline popover draws); a
  // custom cubic, or one mid-drag, draws the live control points.
  const named = !draft && !parseCubic(value) ? CURVE_GLYPHS.find((c) => c.key === value) : undefined;
  const path = springPathOf(value) ?? named?.path ?? curvePath(cur);

  const onDown = (h: 0 | 1) => (e: React.PointerEvent) => {
    e.preventDefault();
    const rect = box.current?.getBoundingClientRect(); if (!rect) return;
    let next = cur;
    setGrab(h); setDraft(cur);
    const move = (ev: PointerEvent) => {
      const [x, y] = clientToCurve(ev.clientX, ev.clientY, rect);
      next = setHandle(cur, h, x, y);
      setDraft(next);
    };
    // Release commits ONE op for the whole drag; repeated drags inside useEditHistory's coalesce
    // window still fold into a single undo step.
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setGrab(null); commit(next);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const onKey = (h: 0 | 1) => (e: React.KeyboardEvent) => {
    const d = NUDGE[e.key]; if (!d) return;
    e.preventDefault();
    commit(nudgeHandle(cur, h, d[0], d[1]));
  };

  // A plain render function, NOT a nested component: a nested one is a new component type every
  // render, so React would remount the dot on each commit and a keyboard nudge would lose focus
  // after its first arrow press.
  const handle = (h: 0 | 1) => {
    const [left, top] = handlePct(cur, h);
    return (
      <motion.button type="button" className={`e-curve-h${grab === h ? " grab" : ""}`}
        animate={{ left: `${left}%`, top: `${top}%` }} transition={still ? { duration: 0 } : HANDLE_SPRING}
        role="slider" aria-label={`${label} control point ${h + 1}`} title="Drag to shape the curve"
        aria-valuemin={0} aria-valuemax={1} aria-valuenow={cur[h * 2]}
        aria-valuetext={`x ${cur[h * 2].toFixed(2)}, y ${cur[h * 2 + 1].toFixed(2)}`}
        onPointerDown={onDown(h)} onKeyDown={onKey(h)} />
    );
  };

  return (
    <>
      <EasingPicker value={value} onChange={onChange} label={label} />
      <div className="e-curve-box" ref={box}>
        <svg viewBox={VIEW_BOX} preserveAspectRatio="none" className="e-curve-svg" aria-hidden="true">
          <line x1="0" y1="0" x2="100" y2="0" className="e-curve-grid" />
          <line x1="0" y1="100" x2="100" y2="100" className="e-curve-grid" />
          {!spr && <>
            <line x1="0" y1="100" x2={cur[0] * 100} y2={(1 - cur[1]) * 100} className="e-curve-arm" />
            <line x1="100" y1="0" x2={cur[2] * 100} y2={(1 - cur[3]) * 100} className="e-curve-arm" />
          </>}
          <path d={path} className="e-curve-path" />
        </svg>
        {!spr && handle(0)}
        {!spr && handle(1)}
      </div>
      {spr && <SpringControls stiffness={spr[0]} damping={spr[1]} mass={spr[2]} onChange={onChange} />}
    </>
  );
}
