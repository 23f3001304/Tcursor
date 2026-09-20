import { useId, type ReactNode } from "react";
import { motion, useReducedMotion } from "motion/react";
import { IconChevronDown, IconChevronUp } from "@tabler/icons-react";
import { NumberField } from "../controls/Controls";
import { PRESS, PRESS_SPRING, secOf } from "./InspectorShape";

const GLIDE = { type: "spring" as const, stiffness: 500, damping: 40 };

const CELL_STEP = 0.05;

export function TimingRow({
  startMs,
  endMs,
  durMs,
  minSpanMs = 0,
  onStart,
  onEnd,
}: {
  startMs: number;
  endMs: number;
  durMs: number;
  minSpanMs?: number;
  onStart: (ms: number) => void;
  onEnd: (ms: number) => void;
}) {
  return (
    <div className="e-field2">
      <label className="e-field">
        <span className="e-fl">Start</span>
        <NumberField
          min={0}
          max={secOf(endMs - minSpanMs)}
          value={secOf(startMs)}
          onChange={(v) => onStart(Math.round(v * 1000))}
        />
      </label>
      <label className="e-field">
        <span className="e-fl">End</span>
        <NumberField
          min={secOf(startMs + minSpanMs)}
          max={secOf(durMs)}
          value={secOf(endMs)}
          onChange={(v) => onEnd(Math.round(v * 1000))}
        />
      </label>
    </div>
  );
}

export interface ValueCell {
  label: string;
  sec: number;
  min: number;
  max: number;
  onChange: (sec: number) => void;
}

export const valueRowColumns = (count: number) => (count > 3 ? 2 : Math.max(1, count));

export function ValueRow({ cells, ariaLabel }: { cells: ValueCell[]; ariaLabel: string }) {
  const cols = valueRowColumns(cells.length);
  return (
    <div
      className="e-ivals"
      role="group"
      aria-label={ariaLabel}
      style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}
    >
      {cells.map((c, i) => {
        const set = (raw: number) => c.onChange(Math.min(c.max, Math.max(c.min, +raw.toFixed(2))));
        return (
          <div
            className="e-ival"
            key={c.label}
            style={{
              boxShadow:
                [
                  i % cols > 0 ? "inset 1px 0 0 var(--e-divider)" : "",
                  i >= cols ? "inset 0 1px 0 var(--e-divider)" : "",
                ]
                  .filter(Boolean)
                  .join(", ") || undefined,
            }}
          >
            <span className="e-ival-l">{c.label}</span>
            <div className="e-ival-row">
              <span className="e-ival-v">
                {c.sec.toFixed(2)}
                <i>s</i>
              </span>
              <span className="e-ival-steps">
                <button
                  type="button"
                  title={`More ${c.label}`}
                  aria-label={`More ${c.label}`}
                  disabled={c.sec >= c.max}
                  onClick={() => set(c.sec + CELL_STEP)}
                >
                  <IconChevronUp size={11} />
                </button>
                <button
                  type="button"
                  title={`Less ${c.label}`}
                  aria-label={`Less ${c.label}`}
                  disabled={c.sec <= c.min}
                  onClick={() => set(c.sec - CELL_STEP)}
                >
                  <IconChevronDown size={11} />
                </button>
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export interface SegOption {
  key: string;
  label: string;
  on: boolean;
  disabled?: boolean;
  title?: string;
  visual?: ReactNode;
}

export function SegRow({
  options,
  onPick,
  thumbs = false,
  ariaLabel,
}: {
  options: SegOption[];
  onPick: (key: string) => void;
  thumbs?: boolean;
  ariaLabel: string;
}) {
  const still = useReducedMotion();
  const id = useId();
  return (
    <div className={`e-preseg${thumbs ? " thumbs" : ""}`} role="group" aria-label={ariaLabel}>
      {options.map((o) => (
        <motion.button
          key={o.key}
          type="button"
          className={`e-preseg-b${o.on ? " on" : ""}`}
          aria-pressed={o.on}
          disabled={o.disabled}
          title={o.title ?? o.label}
          whileTap={o.disabled || still ? undefined : PRESS}
          transition={PRESS_SPRING}
          onClick={() => onPick(o.key)}
        >
          {o.visual}
          <span className="e-preseg-t">{o.label}</span>
          {o.on &&
            (still ? (
              <span className="e-preseg-tick" />
            ) : (
              <motion.span layoutId={`seg-${id}`} className="e-preseg-tick" transition={GLIDE} />
            ))}
        </motion.button>
      ))}
    </div>
  );
}
