import { useId, useState, type ReactNode } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconChevronRight } from "@tabler/icons-react";

const key = (id: string) => `tcursor.panel.more.${id}`;

const TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

export function readDisclosure(id: string): boolean {
  try {
    return localStorage.getItem(key(id)) === "1";
  } catch {
    return false;
  }
}

export function writeDisclosure(id: string, open: boolean): void {
  try {
    localStorage.setItem(key(id), open ? "1" : "0");
  } catch {}
}

export function Disclosure({
  id,
  label = "More",
  children,
}: {
  id: string;
  label?: string;
  children: ReactNode;
}) {
  const bodyId = useId();
  const still = useReducedMotion();
  const [open, setOpen] = useState(() => readDisclosure(id));
  const toggle = () => {
    const next = !open;
    setOpen(next);
    writeDisclosure(id, next);
  };
  const timing = still ? { duration: 0 } : TWEEN;

  return (
    <div className="e-more">
      <button
        type="button"
        className="e-more-btn"
        aria-expanded={open}
        aria-controls={bodyId}
        onClick={toggle}
      >
        <motion.span className="e-more-chev" animate={{ rotate: open ? 90 : 0 }} transition={timing}>
          <IconChevronRight size={13} />
        </motion.span>
        {label}
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            id={bodyId}
            className="e-more-body"
            key="body"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={timing}
          >
            <div className="e-more-inner">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
