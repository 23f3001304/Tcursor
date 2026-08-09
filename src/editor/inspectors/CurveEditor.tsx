import { useEffect, useRef, useState } from "react";
import { formatCubic, parseCubic } from "../../lib/cubicBezier";
import { CAM_CURVES } from "./curves";
import { clientToCurve, curveOf, curvePath, nudgeHandle, setHandle, toSvg, VIEW_BOX, type Cubic } from "./curveMath";

const NUDGE: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1],
};

/** The dashed 0 and 1 value guides every card draws behind its curve. */
const Grid = () => (
  <>
    <line x1="0" y1="0" x2="100" y2="0" stroke="var(--e-border2)" strokeDasharray="3 3" />
    <line x1="0" y1="100" x2="100" y2="100" stroke="var(--e-border2)" strokeDasharray="3 3" />
  </>
);

export function CurveEditor({ value, onChange, label = "Transition Curve" }: {
  value: string; onChange: (easing: string) => void; label?: string;
}) {
  const isCustom = parseCubic(value) !== null;
  // The in-flight drag, so the curve follows the pointer without an op per pointermove. Cleared
  // when the committed `value` comes back from the backend (or immediately, if it is unchanged).
  const [draft, setDraft] = useState<Cubic | null>(null);
  const [grab, setGrab] = useState<0 | 1 | null>(null);
  const svg = useRef<SVGSVGElement>(null);
  useEffect(() => { setDraft(null); }, [value]);

  const cur = draft ?? curveOf(value);
  const commit = (c: Cubic) => onChange(formatCubic(c[0], c[1], c[2], c[3]));

  const onHandleDown = (h: 0 | 1) => (e: React.PointerEvent) => {
    e.preventDefault(); e.stopPropagation();
    const el = svg.current; if (!el) return;
    const rect = el.getBoundingClientRect();
    let next = cur;
    setGrab(h); setDraft(cur);
    const move = (ev: PointerEvent) => {
      const [x, y] = clientToCurve(ev.clientX, ev.clientY, rect);
      next = setHandle(cur, h, x, y);
      setDraft(next);
    };
    // Release commits ONE op for the whole drag. Repeated drags inside useEditHistory's
    // COALESCE_MS window still fold into a single undo step.
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setGrab(null);
      commit(next);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const onHandleKey = (h: 0 | 1) => (e: React.KeyboardEvent) => {
    const d = NUDGE[e.key]; if (!d) return;
    e.preventDefault();
    commit(nudgeHandle(cur, h, d[0], d[1]));
  };

  // A plain render function, NOT a nested component: a nested one would be a new component type
  // every render, so React would remount the handle on each commit and a keyboard nudge would
  // lose focus after its first arrow press.
  const handle = (h: 0 | 1) => {
    const [sx, sy] = toSvg(cur[h * 2], cur[h * 2 + 1]);
    const [ax, ay] = h === 0 ? [0, 100] : [100, 0];
    return (
      <g>
        <line x1={ax} y1={ay} x2={sx} y2={sy} className="e-curve-arm" />
        <circle cx={sx} cy={sy} r="7" className={`e-curve-h${grab === h ? " grab" : ""}`}
          tabIndex={0} role="slider" aria-label={`Curve control point ${h + 1}`}
          aria-valuetext={`x ${cur[h * 2].toFixed(2)}, y ${cur[h * 2 + 1].toFixed(2)}`}
          onPointerDown={onHandleDown(h)} onKeyDown={onHandleKey(h)} />
      </g>
    );
  };

  // The presets, plus a Custom card that exists only while the value IS a custom cubic. Exactly
  // one card is `on`, and that one expands in place into the interactive editor.
  const cards = [
    ...CAM_CURVES.map((c) => ({ key: c.key, name: c.name, path: c.path, on: !isCustom && value === c.key })),
    ...(isCustom ? [{ key: "custom", name: "Custom", path: curvePath(cur), on: true }] : []),
  ];

  return (
    <div className="e-field">
      <span className="e-fl">{label}</span>
      <div className="e-curve-pick">
        {cards.map((c) => c.on ? (
          // A CONSTANT key (not c.key): the first arrow nudge on a preset turns the value custom,
          // which changes which entry is selected - a per-entry key would remount the card there
          // and drop keyboard focus mid-edit.
          <div key="selected" className="e-curve-card on wide">
            <svg ref={svg} viewBox={VIEW_BOX} className="e-curve-svg">
              <Grid />
              <path d={curvePath(cur)} fill="none" stroke="var(--e-fg)" strokeWidth="4" strokeLinecap="round" />
              {handle(0)}
              {handle(1)}
            </svg>
            <span>{c.name}<em>drag the dots</em></span>
          </div>
        ) : (
          <button key={c.key} type="button" className="e-curve-card" onClick={() => onChange(c.key)}>
            <svg viewBox={VIEW_BOX} className="e-curve-svg">
              <Grid />
              <path d={c.path} fill="none" stroke="var(--e-mut)" strokeWidth="6" strokeLinecap="round" />
            </svg>
            <span>{c.name}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
