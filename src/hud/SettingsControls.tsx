import type { ReactNode } from "react";

/** A labelled settings row: label (+ optional value hint) above its control. */
export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="sf">
      <div className="sf-head"><span className="sf-label">{label}</span>{hint && <span className="sf-hint">{hint}</span>}</div>
      {children}
    </div>
  );
}

/** iOS-style on/off switch. */
export function Switch({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button type="button" className={`sw ${on ? "on" : ""}`} role="switch" aria-checked={on} onClick={() => onChange(!on)}>
      <span className="sw-knob" />
    </button>
  );
}

/** Range slider with a formatted value readout. */
export function Slider({ value, min, max, step, onChange, fmt }: {
  value: number; min: number; max: number; step: number; onChange: (v: number) => void; fmt: (v: number) => string;
}) {
  return (
    <div className="sl">
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(parseFloat(e.target.value))} />
      <span className="sl-val">{fmt(value)}</span>
    </div>
  );
}
