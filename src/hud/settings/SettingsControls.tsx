import { useState, useRef, type ReactNode } from "react";
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

/** iOS-style on/off switch. `hsw` (not `sw`): the editor's `Switch` owns `.sw` in
 *  `editor/controls/controls.css` and both sheets ship in the one bundle, so a shared name let the
 *  editor's rule (painted with editor-only tokens the HUD never defines) win here and the track
 *  lost its colour in both states. */
export function Switch({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button type="button" className={`hsw ${on ? "on" : ""}`} role="switch" aria-checked={on} onClick={() => onChange(!on)}>
      <span className="hsw-knob" />
    </button>
  );
}

/** Custom range Slider helper component for the HUD. */
function Slider({
  value,
  min,
  max,
  step = 0.01,
  onChange,
  accentColor = "var(--accent, #ef4444)"
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  accentColor?: string;
}) {
  const trackRef = useRef<HTMLDivElement>(null);

  const handlePointerDown = (e: React.PointerEvent) => {
    if (!trackRef.current) return;
    trackRef.current.setPointerCapture(e.pointerId);
    updateValue(e.clientX);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!trackRef.current || !trackRef.current.hasPointerCapture(e.pointerId)) return;
    updateValue(e.clientX);
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    if (trackRef.current && trackRef.current.hasPointerCapture(e.pointerId)) {
      trackRef.current.releasePointerCapture(e.pointerId);
    }
  };

  const updateValue = (clientX: number) => {
    if (!trackRef.current) return;
    const rect = trackRef.current.getBoundingClientRect();
    const pct = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    const rawVal = min + pct * (max - min);
    
    const stepsCount = Math.round((rawVal - min) / step);
    const steppedVal = Math.max(min, Math.min(max, min + stepsCount * step));
    
    const stepStr = step.toString();
    const decimalIndex = stepStr.indexOf(".");
    const precision = decimalIndex === -1 ? 0 : stepStr.length - decimalIndex - 1;
    
    onChange(Number(steppedVal.toFixed(precision)));
  };

  const pct = Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100));

  return (
    <div
      ref={trackRef}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      style={{
        position: "relative",
        height: 20,
        display: "flex",
        alignItems: "center",
        cursor: "pointer",
        userSelect: "none",
        width: "100%",
        touchAction: "none"
      }}
    >
      <div
        style={{
          width: "100%",
          height: 6,
          borderRadius: 999,
          background: "var(--hover, #f4f4f5)",
          border: "1px solid var(--line, rgba(0, 0, 0, 0.08))",
          position: "relative"
        }}
      >
        <div
          style={{
            position: "absolute",
            left: 0,
            top: -1,
            bottom: -1,
            width: `${pct}%`,
            background: accentColor,
            borderRadius: 999
          }}
        />
        <motion.div
          whileHover={{ scale: 1.25 }}
          whileTap={{ scale: 0.95 }}
          transition={{ type: "spring", stiffness: 400, damping: 25 }}
          style={{
            position: "absolute",
            left: `calc(${pct}% - 8px)`,
            top: -5,
            width: 14,
            height: 14,
            borderRadius: "50%",
            background: "#fff",
            border: `2px solid ${accentColor}`,
            boxShadow: "0 2px 4px rgba(0,0,0,0.2)"
          }}
        />
      </div>
    </div>
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
      <Slider min={min} max={max} step={step} value={value} onChange={onChange} />
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
