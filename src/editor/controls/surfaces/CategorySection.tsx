import { useId, useState, type ReactNode } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconChevronRight } from "@tabler/icons-react";

const key = (id: string) => `tcursor.panel.cat.${id}`;

const TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

export function readCategory(id: string): boolean | null {
  try {
    const v = localStorage.getItem(key(id));
    return v === null ? null : v === "1";
  } catch {
    return null;
  }
}

export function writeCategory(id: string, open: boolean): void {
  try {
    localStorage.setItem(key(id), open ? "1" : "0");
  } catch {}
}

export function defaultOpenIndex(holdsSelection: boolean[]): number {
  const i = holdsSelection.indexOf(true);
  return i >= 0 ? i : 0;
}

export function CategorySection({
  id,
  label,
  count,
  selectedName = null,
  defaultOpen,
  children,
}: {
  id: string;
  label: string;
  count: number;
  selectedName?: string | null;
  defaultOpen: boolean;
  children: ReactNode;
}) {
  const bodyId = useId();
  const still = useReducedMotion();
  const [open, setOpen] = useState(() => readCategory(id) ?? defaultOpen);
  const toggle = () => {
    const next = !open;
    setOpen(next);
    writeCategory(id, next);
  };
  const timing = still ? { duration: 0 } : TWEEN;

  return (
    <div className="e-cat">
      <button
        type="button"
        className="e-cat-btn"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={toggle}
      >
        <motion.span className="e-cat-chev" animate={{ rotate: open ? 90 : 0 }} transition={timing}>
          <IconChevronRight size={13} />
        </motion.span>
        <span className="e-cat-label">{label}</span>
        <span className="e-cat-count">{count}</span>
        {!open && selectedName && <span className="e-cat-sel">{selectedName}</span>}
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            id={bodyId}
            className="e-cat-body"
            key="body"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={timing}
          >
            <div className="e-cat-inner">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
