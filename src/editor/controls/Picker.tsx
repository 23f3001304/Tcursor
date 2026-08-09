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
  options: { value: T; label: string }[];
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
  const currentLabel = options.find((o) => o.value === value)?.label ?? value;

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
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-activedescendant={open && activeIndex >= 0 ? `${uid}-opt-${activeIndex}` : undefined}
        onClick={() => (open ? closeMenu() : openMenu())}
        onKeyDown={handleKeyDown}
        style={{
          width: "100%",
          height: 36,
          background: "var(--e-soft)",
          border: "1px solid var(--e-border)",
          borderRadius: "var(--e-r)",
          padding: "0 12px",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          cursor: "pointer",
          color: "var(--e-fg)",
          fontFamily: "inherit",
          fontSize: 13,
          fontWeight: 500,
          textAlign: "left",
          outline: "none",
          boxSizing: "border-box"
        }}
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
            style={{
              position: "absolute",
              top: "100%",
              left: 0,
              right: 0,
              zIndex: 50,
              background: "var(--e-card)",
              border: "1px solid var(--e-border)",
              borderRadius: "var(--e-r)",
              boxShadow: "var(--e-shadow-pop)",
              padding: 4,
              maxHeight: 200,
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
                aria-selected={value === opt.value}
                onClick={() => { onChange(opt.value); closeMenu(); }}
                style={{
                  width: "100%",
                  height: 32,
                  padding: "0 8px",
                  display: "flex",
                  alignItems: "center",
                  background: value === opt.value ? "var(--e-soft)" : "transparent",
                  color: "var(--e-fg)",
                  border: "none",
                  borderRadius: "var(--e-r-sm)",
                  cursor: "pointer",
                  fontSize: 12.5,
                  fontWeight: value === opt.value ? 600 : 500,
                  textAlign: "left",
                  outline: i === activeIndex ? "2px solid var(--e-focus)" : "none",
                  outlineOffset: -2,
                  transition: "background-color 0.14s ease, color 0.14s ease"
                }}
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
