import { useId, useState, type ReactNode } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconChevronRight } from "@tabler/icons-react";

// The panel pass's one escape valve for a panel whose content cannot fit 620px: ONE quiet row per
// panel holding its least-used controls. One, not several - a panel of collapsed sections is a
// filing cabinet, and the point of this pass was to stop making people hunt.
//
// It is deliberately not a card: no plane, no stroke, no fill. A dim 11px word and a chevron, the
// same weight as a section heading, so a closed panel reads as finished rather than truncated.

const key = (id: string) => `tcursor.panel.more.${id}`;
// The 0.16s content-swap tween from the m1a Look paragraph - the panel's one non-spring motion.
const TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

/** Is this panel's disclosure remembered as open? Storage can throw outright (a webview with site
 *  data blocked), and a panel must still render, so every failure reads as closed. */
export function readDisclosure(id: string): boolean {
  try { return localStorage.getItem(key(id)) === "1"; } catch { return false; }
}

/** Remember this panel's disclosure state. Silent on failure, for the same reason. */
export function writeDisclosure(id: string, open: boolean): void {
  try { localStorage.setItem(key(id), open ? "1" : "0"); } catch { /* nothing to remember with */ }
}

export function Disclosure({ id, label = "More", children }: {
  /** Storage key suffix - one per panel ("background", "cursor", ...), so panels remember apart. */
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
  // Under reduced motion the body still enters and leaves, just instantly: a zero-duration tween
  // rather than a dropped animation, so `AnimatePresence` keeps owning the unmount either way.
  const timing = still ? { duration: 0 } : TWEEN;

  return (
    <div className="e-more">
      <button type="button" className="e-more-btn" aria-expanded={open} aria-controls={bodyId} onClick={toggle}>
        <motion.span className="e-more-chev" animate={{ rotate: open ? 90 : 0 }} transition={timing}>
          <IconChevronRight size={13} />
        </motion.span>
        {label}
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div id={bodyId} className="e-more-body" key="body"
            initial={{ height: 0, opacity: 0 }} animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }} transition={timing}>
            {/* The gap above the first row lives on an INNER element: animating the outer box's
                height while its own padding changes would make the content jump at both ends. */}
            <div className="e-more-inner">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
