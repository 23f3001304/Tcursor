import { useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="sf">
      <div className="sf-head">
        <span className="sf-label">{label}</span>
        {hint && <span className="sf-hint">{hint}</span>}
      </div>
      {children}
    </div>
  );
}

export function Switch({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      className={`hsw ${on ? "on" : ""}`}
      role="switch"
      aria-checked={on}
      onClick={() => onChange(!on)}
    >
      <span className="hsw-knob" />
    </button>
  );
}

export function Advanced({
  children,
  open: openProp,
  onToggle,
}: {
  children: ReactNode;
  open?: boolean;
  onToggle?: (v: boolean) => void;
}) {
  const [openS, setOpenS] = useState(false);
  const open = openProp ?? openS;
  const setOpen = (v: boolean) => {
    if (onToggle) onToggle(v);
    else setOpenS(v);
  };
  return (
    <div className="adv">
      <button
        type="button"
        className={`adv-tog ${open ? "open" : ""}`}
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        Advanced <span className="adv-caret">{"›"}</span>
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            className="adv-body"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.18, ease: "easeOut" }}
            style={{ overflow: "hidden" }}
          >
            {children}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
