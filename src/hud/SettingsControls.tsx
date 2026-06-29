import { useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";

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

/** Compact numeric control: label left, value once on the right, slim slider below.
 *  Optional hint renders next to the value (muted, same .sf-hint style). */
export function Range({ label, value, min, max, step, onChange, fmt, hint }: {
  label: string; value: number; min: number; max: number; step: number; onChange: (v: number) => void; fmt: (v: number) => string; hint?: string;
}) {
  return (
    <label className="rng">
      <span className="rng-head">
        <span className="rng-label">{label}</span>
        <span style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
          {hint && <span className="sf-hint">{hint}</span>}
          <span className="rng-val">{fmt(value)}</span>
        </span>
      </span>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(parseFloat(e.target.value))} />
    </label>
  );
}

/** Collapsible "Advanced" section (closed by default; Motion height/opacity expand). */
export function Advanced({ children, open: openProp, onToggle }: { children: ReactNode; open?: boolean; onToggle?: (v: boolean) => void }) {
  const [openS, setOpenS] = useState(false);
  const open = openProp ?? openS;
  const setOpen = (v: boolean) => { if (onToggle) onToggle(v); else setOpenS(v); };
  return (
    <div className="adv">
      <button type="button" className={`adv-tog ${open ? "open" : ""}`} aria-expanded={open} onClick={() => setOpen(!open)}>
        Advanced <span className="adv-caret">{"›"}</span>
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div className="adv-body" initial={{ height: 0, opacity: 0 }} animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }} transition={{ duration: 0.18, ease: "easeOut" }} style={{ overflow: "hidden" }}>
            {children}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
