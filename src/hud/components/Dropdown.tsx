import { useEffect, useRef, type ReactNode } from "react";
import { Chevron, Check } from "./icons";

export interface DropOption { id: string; label: string }

/** Custom select menu. The HUD window resizes taller while one is open so the
 *  menu (which overflows the bar's window) is visible. */
export function Dropdown({ icon, value, options, open, onToggle, onPick }: {
  icon?: ReactNode;
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

  const label = options.find((o) => o.id === value)?.label ?? options[0]?.label ?? "—";

  return (
    <div className={`dd ${open ? "open" : ""}`} ref={ref}>
      <button className="dd-trigger" onClick={onToggle}>
        {icon && <span className="ico">{icon}</span>}
        <span className="dd-label">{label}</span>
        <span className="chev"><Chevron /></span>
      </button>
      {open && (
        <div className="dd-menu">
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
        </div>
      )}
    </div>
  );
}
