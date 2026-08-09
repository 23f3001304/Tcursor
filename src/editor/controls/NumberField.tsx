import { type CSSProperties } from "react";
import { IconPlus, IconMinus } from "@tabler/icons-react";

// Number input with +/- steppers. The stepper button look (hover/disabled/active) lives in
// .e-numstep (editor.css) so they get a proper affordance instead of a flat transparent icon.
// Deliberately no free-typed text entry: the value only ever changes by a clamped `step`, so
// there is no empty/negative intermediate state a caller's `onChange` could ever see mid-edit.
export function NumberField({ value, min = 0, max, step = 0.1, unit = "s", onChange }: {
  value: number; min?: number; max?: number; step?: number; unit?: string; onChange: (v: number) => void;
}) {
  const increment = () => { const nv = +(value + step).toFixed(2); if (max === undefined || nv <= max) onChange(nv); };
  const decrement = () => { const nv = +(value - step).toFixed(2); if (nv >= min) onChange(nv); };
  const btn: CSSProperties = { width: 26, height: 26, borderRadius: "var(--e-r-sm)", border: "none",
    display: "flex", alignItems: "center", justifyContent: "center", color: "var(--e-fg)", flex: "none" };
  return (
    <div style={{ display: "flex", alignItems: "center", background: "var(--e-soft)", border: "1px solid var(--e-border)",
      borderRadius: "var(--e-r)", height: 36, padding: "0 5px", width: "100%", boxSizing: "border-box" }}>
      <button type="button" className="e-numstep" onClick={decrement} disabled={value <= min} style={btn}><IconMinus size={13} /></button>
      <span style={{ flex: 1, minWidth: 0, textAlign: "center", color: "var(--e-fg)", fontSize: 13, fontWeight: 600,
        fontVariantNumeric: "tabular-nums", userSelect: "none" }}>
        {value}{unit && <span style={{ color: "var(--e-mut)", fontWeight: 500, marginLeft: 2 }}>{unit}</span>}
      </span>
      <button type="button" className="e-numstep" onClick={increment} disabled={max !== undefined && value >= max} style={btn}><IconPlus size={13} /></button>
    </div>
  );
}
