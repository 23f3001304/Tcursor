import { useRef, useState } from "react";
import { snapToStep } from "./Slider";

/** The number a typed slider readout should commit to, or `null` when the text holds no number
 *  at all (so the caller can leave the value alone instead of writing a 0). Units the formatter
 *  added are tolerated - "35 %", "1.20x", "160 deg", "-40 ms" all parse - because the field opens
 *  pre-filled with the bare number but a user may well type the unit back in. The result is
 *  already snapped and clamped, so a typed value can never land off-step or out of range. */
export function parseSliderInput(text: string, min: number, max: number, step: number): number | null {
  const m = /-?\d*\.?\d+/.exec(text.replace(/\s+/g, ""));
  if (!m) return null;
  const raw = Number(m[0]);
  return Number.isFinite(raw) ? snapToStep(raw, min, max, step) : null;
}

/** A slider's label and its live value on ONE row: name left, value right, tabular figures so the
 *  digits never dance while dragging. Clicking the value swaps it for a small input - Enter or
 *  blur commits, Esc reverts - which is the only way to set an exact number on a 320px track. */
export function SliderValue({ label, value, text, min, max, step, disabled, onCommit }: {
  label: string;
  /** The live (drag-tracking) number, used as the edit field's starting text. */
  value: number;
  /** That number already run through the caller's `formatValue`, e.g. `"35%"`. */
  text: string;
  min: number; max: number; step: number;
  disabled?: boolean;
  onCommit: (v: number) => void;
}) {
  const [editing, setEditing] = useState(false);
  // Esc sets this before blurring, so the blur that follows commits nothing.
  const reverting = useRef(false);

  const commit = (raw: string) => {
    const next = parseSliderInput(raw, min, max, step);
    if (next !== null && next !== value) onCommit(next);
  };

  return (
    <span className="e-fl">
      {/* The literal space is what the old one-string markup (`{label} <b>{value}</b>`) put
          between the two, and callers' tests read this row as text ("Factor 2x"). A whitespace-only
          text node is never rendered as a flex item, so it costs nothing in layout. */}
      <span className="e-fl-name">{label}</span>{" "}
      {editing ? (
        <input
          className="e-val-edit"
          type="text"
          defaultValue={String(value)}
          aria-label={`${label} value`}
          autoFocus
          onFocus={(e) => e.currentTarget.select()}
          onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); commit(e.currentTarget.value); setEditing(false); }
            else if (e.key === "Escape") { e.preventDefault(); reverting.current = true; setEditing(false); }
          }}
          onBlur={(e) => {
            if (!reverting.current) commit(e.currentTarget.value);
            reverting.current = false;
            setEditing(false);
          }}
        />
      ) : (
        <button type="button" className="e-val" disabled={disabled} title={`${label}: click to type a value`}
          onClick={() => setEditing(true)}>{text}</button>
      )}
    </span>
  );
}
