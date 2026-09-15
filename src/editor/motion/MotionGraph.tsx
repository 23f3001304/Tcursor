import { GHOST_PX, type GraphInput, type GraphRamp } from "./graphModel";
import { useGraphDrag } from "./useGraphDrag";
import type { GraphPatch } from "./graphEdits";
import "./motion.css";

const LANE_LABEL = { zoom: "Zoom", layout: "Layout", cam: "Camera move" } as const;

export function MotionGraph({
  input,
  onCommit,
  readOnly = false,
  retimeable = true,
  className,
}: {
  input: GraphInput;
  onCommit: (patch: GraphPatch) => void;
  readOnly?: boolean;
  retimeable?: boolean;
  className?: string;
}) {
  const { model, selected, handlers } = useGraphDrag(input, onCommit, { readOnly, retimeable });
  const p = model.plot;
  const left = p.x - GHOST_PX,
    right = p.x + p.w + GHOST_PX;

  const keyNodes = (r: GraphRamp) =>
    (r.keys ?? []).map((k, n, all) => {
      const showIn = n > 0 && all[n - 1].mode === "b";
      const showOut = n < all.length - 1 && k.mode === "b";
      const on = selected?.ramp === r.which && selected.key === n;
      return (
        <g key={n} className={`e-mg-key${on ? " on" : ""}`}>
          {showIn && (
            <>
              <line className="e-mg-arm" x1={k.x} y1={k.y} x2={k.inX} y2={k.inY} />
              <circle className="e-mg-h" cx={k.inX} cy={k.inY} r={3} {...handlers.handle(r.which, n, "in")} />
            </>
          )}
          {showOut && (
            <>
              <line className="e-mg-arm" x1={k.x} y1={k.y} x2={k.outX} y2={k.outY} />
              <circle
                className="e-mg-h"
                cx={k.outX}
                cy={k.outY}
                r={3}
                {...handlers.handle(r.which, n, "out")}
              />
            </>
          )}
          <circle className="e-mg-dot" cx={k.x} cy={k.y} r={4} {...handlers.key(r.which, n)} />
        </g>
      );
    });

  return (
    <svg
      {...handlers.svg}
      viewBox={`0 0 ${model.width} ${model.height}`}
      className={`e-mg e-mg-${input.lane}${className ? ` ${className}` : ""}`}
      role="group"
      aria-label={`${LANE_LABEL[input.lane]} motion, drag a key to shape it`}
    >
      {model.yTicks.map((t) => (
        <g key={t.label}>
          <line className="e-mg-rail" x1={left} x2={right} y1={t.y} y2={t.y} />
          <text className="e-mg-tick" x={left - 4} y={t.y + 3} textAnchor="end">
            {t.label}
          </text>
        </g>
      ))}
      {model.ticks.map((t) => (
        <text key={t.label} className="e-mg-tick" x={t.x} y={model.height - 5} textAnchor="middle">
          {t.label}
        </text>
      ))}
      {model.ramps.map((r) => (
        <rect
          key={`band-${r.which}`}
          className="e-mg-band"
          x={Math.min(r.x0, r.x1)}
          y={p.y}
          width={Math.abs(r.x1 - r.x0)}
          height={p.h}
        />
      ))}
      {model.plateau && (
        <line
          className="e-mg-hold"
          x1={model.plateau.x0}
          x2={model.plateau.x1}
          y1={model.plateau.y}
          y2={model.plateau.y}
        />
      )}
      {model.hint && (
        <text
          className="e-mg-hint"
          x={(model.hint.x0 + model.hint.x1) / 2}
          y={model.hint.y + 13}
          textAnchor="middle"
        >
          follows the cursor
        </text>
      )}
      {model.ghosts.map((d, i) => (
        <path key={`ghost-${i}`} className="e-mg-ghost" d={d} />
      ))}
      {model.ramps.map((r) => (
        <path key={`curve-${r.which}`} className="e-mg-curve" d={r.path} />
      ))}
      {!readOnly &&
        model.ramps.map((r) => (r.editable && r.keys ? <g key={`keys-${r.which}`}>{keyNodes(r)}</g> : null))}
    </svg>
  );
}
