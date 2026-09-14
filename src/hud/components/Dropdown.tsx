import { useEffect, useRef, type ReactNode } from "react";
import { motion } from "motion/react";
import { Chevron, Check } from "./icons";

export interface DropOption { id: string; label: string }

// design/premium-pass D6: the app-wide press spring for the trigger, plus a Motion mount-in for
// the menu (replaces the CSS `menu-in` keyframe 1:1). Deliberately enter-ONLY, no AnimatePresence:
// `useHudWindowSize` (Hud.tsx) shrinks the actual OS window back to 132px tall the instant `menu`
// goes null, synchronously with this closing - an AnimatePresence exit would animate the menu
// fading out UNDERNEATH a window that's already shrinking around it, getting visibly clipped
// instead of fading cleanly (the card's own body states never hit this race: they all swap inside
// one window size, so nothing is shrinking around them - only a menu overflows the frame).
// Closing stays a hard cut, matching the pre-D6 behavior exactly.
const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
const MENU_MOTION = { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, transition: { duration: 0.14 } };

/** Custom select menu. The HUD window resizes taller while one is open so the
 *  menu (which overflows the bar's window) is visible. */
export function Dropdown({ icon, value, options, open, onToggle, onPick, row }: {
  icon?: ReactNode;
  /** The idle card's full-width row (`.dd-row`) instead of the compact ghost trigger. */
  row?: boolean;
  value: string;
  options: DropOption[];
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

  const label = options.find((o) => o.id === value)?.label ?? options[0]?.label ?? "-";

  return (
    <div className={`dd ${open ? "open" : ""}`} ref={ref}>
      <motion.button className={row ? "dd-row" : "dd-trigger"} onClick={onToggle} whileTap={PRESS_TAP} transition={PRESS_SPRING}>
        {icon && <span className="ico">{icon}</span>}
        <span className="dd-label">{label}</span>
        <span className="chev"><Chevron /></span>
      </motion.button>
      {open && (
        <motion.div className="dd-menu" {...MENU_MOTION}>
          {options.map((o) => (
            <button
              key={o.id}
              className={`dd-item ${o.id === value ? "sel" : ""}`}
              onClick={() => onPick(o.id)}
            >
              <span className="dd-item-label">{o.label}</span>
              {o.id === value && <span className="dd-check"><Check /></span>}
            </button>
          ))}
        </motion.div>
      )}
    </div>
  );
}
