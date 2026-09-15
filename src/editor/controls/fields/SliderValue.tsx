import { useRef, useState } from "react";
import { snapToStep } from "./Slider";

export function parseSliderInput(text: string, min: number, max: number, step: number): number | null {
  const m = /-?\d*\.?\d+/.exec(text.replace(/\s+/g, ""));
  if (!m) return null;
  const raw = Number(m[0]);
  return Number.isFinite(raw) ? snapToStep(raw, min, max, step) : null;
}

export function SliderValue({
  label,
  value,
  text,
  min,
  max,
  step,
  disabled,
  onCommit,
}: {
  label: string;
  value: number;
  text: string;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  onCommit: (v: number) => void;
}) {
  const [editing, setEditing] = useState(false);
  const reverting = useRef(false);

  const commit = (raw: string) => {
    const next = parseSliderInput(raw, min, max, step);
    if (next !== null && next !== value) onCommit(next);
  };

  return (
    <span className="e-fl">
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
            if (e.key === "Enter") {
              e.preventDefault();
              commit(e.currentTarget.value);
              setEditing(false);
            } else if (e.key === "Escape") {
              e.preventDefault();
              reverting.current = true;
              setEditing(false);
            }
          }}
          onBlur={(e) => {
            if (!reverting.current) commit(e.currentTarget.value);
            reverting.current = false;
            setEditing(false);
          }}
        />
      ) : (
        <button
          type="button"
          className="e-val"
          disabled={disabled}
          title={`${label}: click to type a value`}
          onClick={() => setEditing(true)}
        >
          {text}
        </button>
      )}
    </span>
  );
}
