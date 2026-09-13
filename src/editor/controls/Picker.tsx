import { useState, useRef, useEffect, useId } from "react";
import { motion, AnimatePresence } from "motion/react";
import { IconChevronDown } from "@tabler/icons-react";

/** The option index ArrowUp/ArrowDown should move to, or `null` if the key isn't one of those
 *  two. Clamped to `[0, length-1]`; from "nothing active yet" (`activeIndex < 0`), ArrowDown
 *  starts at the first option and ArrowUp at the last (the usual listbox-open convention). */
export function pickerNextIndex(key: string, activeIndex: number, length: number): number | null {
  if (length === 0) return null;
  if (key === "ArrowDown") return activeIndex < 0 ? 0 : Math.min(length - 1, activeIndex + 1);
  if (key === "ArrowUp") return activeIndex < 0 ? length - 1 : Math.max(0, activeIndex - 1);
  return null;
}

// 2. Custom dropdown select Picker component
export function Picker<T extends string>({
  value,
  options,
  onChange,
  ariaLabel
}: {
  value: T;
  /** `title` is optional per-option hover text - e.g. AiPanel puts the full raw model id here
   *  while `label` shows a short derived name (ux audit #16). */
  options: { value: T; label: string; title?: string }[];
  onChange: (v: T) => void;
  ariaLabel?: string;
}) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const uid = useId();

  useEffect(() => {
    const click = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", click);
    return () => document.removeEventListener("mousedown", click);
  }, []);

  const currentIndex = options.findIndex((o) => o.value === value);
  const currentOption = options.find((o) => o.value === value);
  const currentLabel = currentOption?.label ?? value;

  const openMenu = () => { setOpen(true); setActiveIndex(currentIndex >= 0 ? currentIndex : 0); };
  const closeMenu = () => { setOpen(false); setActiveIndex(-1); buttonRef.current?.focus(); };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") { if (open) { e.preventDefault(); closeMenu(); } return; }
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (!open) openMenu();
      else if (activeIndex >= 0) { onChange(options[activeIndex].value); closeMenu(); }
      return;
    }
    const next = pickerNextIndex(e.key, open ? activeIndex : -1, options.length);
    if (next === null) return;
    e.preventDefault();
    if (!open) openMenu(); else setActiveIndex(next);
  };

  return (
    <div ref={containerRef} style={{ position: "relative", width: "100%" }}>
      <button
        ref={buttonRef}
        type="button"
        className="e-picker-btn"
        title={currentOption?.title}
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-activedescendant={open && activeIndex >= 0 ? `${uid}-opt-${activeIndex}` : undefined}
        onClick={() => (open ? closeMenu() : openMenu())}
        onKeyDown={handleKeyDown}
      >
        <span>{currentLabel}</span>
        <motion.span animate={{ rotate: open ? 180 : 0 }} transition={{ duration: 0.15 }}>
          <IconChevronDown size={15} style={{ opacity: 0.6 }} />
        </motion.span>
      </button>

      <AnimatePresence>
        {open && (
          <motion.div
            className="e-picker-menu"
            role="listbox"
            aria-label={ariaLabel}
            initial={{ opacity: 0, y: -4, scale: 0.98 }}
            animate={{ opacity: 1, y: 4, scale: 1 }}
            exit={{ opacity: 0, y: -4, scale: 0.98 }}
            transition={{ duration: 0.12 }}
            // Paint (plane, radius, shadow) lives in `.e-picker-menu` (controls.css); only the
            // popover's placement and scroll box stay here.
            style={{
              position: "absolute",
              top: "100%",
              left: 0,
              right: 0,
              zIndex: 50,
              padding: 4,
              maxHeight: 216,
              overflowY: "auto",
              boxSizing: "border-box"
            }}
          >
            {options.map((opt, i) => (
              <button
                key={opt.value}
                id={`${uid}-opt-${i}`}
                type="button"
                role="option"
                title={opt.title}
                aria-selected={value === opt.value}
                onClick={() => { onChange(opt.value); closeMenu(); }}
                className={`e-picker-opt${value === opt.value ? " on" : ""}`}
                style={{ outline: i === activeIndex ? "2px solid var(--e-focus)" : "none", outlineOffset: -2 }}
              >
                {opt.label}
              </button>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
