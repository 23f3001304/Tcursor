import { useId, useState, type ReactNode } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconChevronRight } from "@tabler/icons-react";

// One collapsible CATEGORY inside a panel: a quiet header row, and the items of that category
// under it. It is `Disclosure`'s idiom (no plane, no stroke, a chevron and an 11px word) turned
// into something a panel may hold several of, because a picture library IS several groups and the
// alternative the owner rejected was making each group scroll sideways.
//
// The rule that keeps a stack of these from being a filing cabinet: the section holding what is
// currently chosen opens by itself, and a closed section that holds the selection says so at the
// right of its header - so the user never has to open a section to find out what they picked.

const key = (id: string) => `tcursor.panel.cat.${id}`;
// The same 0.16s content-swap tween `Disclosure` uses - one collapse language in the panels.
const TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

/** This section's remembered open state, or `null` when the user has never toggled it (which is
 *  what lets `defaultOpen` follow the selection instead of a stale flag). Storage can throw
 *  outright (a webview with site data blocked), and a panel must still render, so every failure
 *  reads as "nothing remembered". */
export function readCategory(id: string): boolean | null {
  try {
    const v = localStorage.getItem(key(id));
    return v === null ? null : v === "1";
  } catch { return null; }
}

/** Remember this section's open state. Silent on failure, for the same reason. */
export function writeCategory(id: string, open: boolean): void {
  try { localStorage.setItem(key(id), open ? "1" : "0"); } catch { /* nothing to remember with */ }
}

/** Which section a panel opens when nothing is remembered: the one holding the current selection,
 *  and the first when no section holds it. `holdsSelection[i]` is "section i contains what is
 *  chosen". Always a valid index for a non-empty list, so a caller can compare `i === index`. */
export function defaultOpenIndex(holdsSelection: boolean[]): number {
  const i = holdsSelection.indexOf(true);
  return i >= 0 ? i : 0;
}

export function CategorySection({ id, label, count, selectedName = null, defaultOpen, children }: {
  /** Storage key suffix - one per category ("cursorpack.Playful", "bg.Ribbons", ...). */
  id: string;
  label: string;
  /** How many items are inside, shown as a dim badge so a closed section still has a size. */
  count: number;
  /** The chosen item's name when THIS section holds the selection, else `null`. Shown only while
   *  closed: open, the ring on the tile says it better. */
  selectedName?: string | null;
  /** Open on a first mount, before the user has ever toggled this section. */
  defaultOpen: boolean;
  children: ReactNode;
}) {
  const bodyId = useId();
  const still = useReducedMotion();
  // Remembered state wins; otherwise the caller's default. Only a TOGGLE writes, so a section the
  // user has never touched keeps following the selection from mount to mount.
  const [open, setOpen] = useState(() => readCategory(id) ?? defaultOpen);
  const toggle = () => {
    const next = !open;
    setOpen(next);
    writeCategory(id, next);
  };
  // Under reduced motion the body still enters and leaves, just instantly: a zero-duration tween
  // rather than a dropped animation, so `AnimatePresence` keeps owning the unmount either way.
  const timing = still ? { duration: 0 } : TWEEN;

  return (
    <div className="e-cat">
      <button type="button" className="e-cat-btn" aria-expanded={open} aria-controls={bodyId} onClick={toggle}>
        <motion.span className="e-cat-chev" animate={{ rotate: open ? 90 : 0 }} transition={timing}>
          <IconChevronRight size={13} />
        </motion.span>
        <span className="e-cat-label">{label}</span>
        <span className="e-cat-count">{count}</span>
        {!open && selectedName && <span className="e-cat-sel">{selectedName}</span>}
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div id={bodyId} className="e-cat-body" key="body"
            initial={{ height: 0, opacity: 0 }} animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }} transition={timing}>
            {/* The gap above the first tile lives on an INNER element: animating the outer box's
                height while its own padding changes would make the content jump at both ends. */}
            <div className="e-cat-inner">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
