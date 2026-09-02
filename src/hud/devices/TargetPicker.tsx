import { useEffect, useRef } from "react";
import { motion } from "motion/react";
import { Chevron, Check, Monitor } from "../components/icons";
import { parseTarget, isOwnProcessWindow, type DisplayInfo } from "./selectDevices";

// design/premium-pass D6: same trigger press spring + menu mount-in as Dropdown.tsx (this
// component is a `Dropdown`-alike, see the doc comment below) - kept local rather than shared to
// match how PLAY_SPRING/PRESS_TAP are hoisted per-file elsewhere in this pass. Deliberately
// enter-ONLY (no AnimatePresence): see Dropdown.tsx's comment for why an exit animation here would
// race `useHudWindowSize` shrinking the actual OS window the instant `menu` goes null.
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
const MENU_MOTION = { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, transition: { duration: 0.14 } };

/** Capture-target picker: a `Dropdown`-alike for `listDisplays()` results, but decorates each
 *  row with its resolution and a "Primary" badge for the main display, and groups `kind:
 *  "window"` targets under their own header - purely presentational, the target_id values and
 *  selection wiring are unchanged from the plain dropdown this replaces. */
export function TargetPicker({ targets, value, open, onToggle, onPick }: {
  targets: DisplayInfo[];
  value: string;
  open: boolean;
  onToggle: () => void;
  onPick: (id: string) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onToggle();
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [open, onToggle]);

  // `index` is computed against the RAW (unfiltered) list first - parseTarget's `primary` flag is
  // keyed off raw index 0, always a display - then this process's own phantom window (see
  // `isOwnProcessWindow`) is dropped from the rendered rows.
  const rows = targets.map((t, i) => ({ t, meta: parseTarget(t, i) })).filter((r) => !isOwnProcessWindow(r.t));
  const screens = rows.filter((r) => r.t.kind !== "window");
  const windows = rows.filter((r) => r.t.kind === "window");
  const selected = rows.find((r) => r.t.id === value);

  const row = (r: (typeof rows)[number]) => (
    <button key={r.t.id} className={`dd-item tp-item ${r.t.id === value ? "sel" : ""}`} onClick={() => onPick(r.t.id)}>
      <span className="tp-main">
        <span className="dd-item-label">{r.meta.title}</span>
        {r.meta.resolution && <span className="tp-res">{r.meta.resolution}</span>}
      </span>
      <span className="tp-trail">
        {r.meta.primary && <span className="tp-badge">Primary</span>}
        {r.t.id === value && <span className="dd-check"><Check /></span>}
      </span>
    </button>
  );

  return (
    <div className={`dd ${open ? "open" : ""}`} ref={ref}>
      <motion.button className="dd-trigger" onClick={onToggle} whileTap={PRESS_TAP} transition={PRESS_SPRING}>
        <span className="ico"><Monitor /></span>
        <span className="dd-label">{selected?.meta.title ?? "—"}</span>
        <span className="chev"><Chevron /></span>
      </motion.button>
      {open && (
        <motion.div className="dd-menu tp-menu" {...MENU_MOTION}>
          {screens.map(row)}
          {windows.length > 0 && <div className="tp-divider">Windows</div>}
          {windows.map(row)}
        </motion.div>
      )}
    </div>
  );
}
