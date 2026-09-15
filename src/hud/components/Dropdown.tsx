import { useEffect, useRef, type ReactNode } from "react";
import { motion } from "motion/react";
import { Chevron, Check } from "./icons";

export interface DropOption {
  id: string;
  label: string;
}

const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };
const MENU_MOTION = {
  initial: { opacity: 0, y: -4 },
  animate: { opacity: 1, y: 0 },
  transition: { duration: 0.14 },
};

export function Dropdown({
  icon,
  value,
  options,
  open,
  onToggle,
  onPick,
  row,
}: {
  icon?: ReactNode;
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
      <motion.button
        className={row ? "dd-row" : "dd-trigger"}
        onClick={onToggle}
        whileTap={PRESS_TAP}
        transition={PRESS_SPRING}
      >
        {icon && <span className="ico">{icon}</span>}
        <span className="dd-label">{label}</span>
        <span className="chev">
          <Chevron />
        </span>
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
              {o.id === value && (
                <span className="dd-check">
                  <Check />
                </span>
              )}
            </button>
          ))}
        </motion.div>
      )}
    </div>
  );
}
