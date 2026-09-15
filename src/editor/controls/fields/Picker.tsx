import { useState, useRef, useEffect, useLayoutEffect, useId } from "react";
import { createPortal } from "react-dom";
import { motion, AnimatePresence } from "motion/react";
import { IconChevronDown } from "@tabler/icons-react";
import { placeStacked, portalHost, type Placement } from "../surfaces/popoverPlace";

const MENU_MAX_H = 216;

export function pickerNextIndex(key: string, activeIndex: number, length: number): number | null {
  if (length === 0) return null;
  if (key === "ArrowDown") return activeIndex < 0 ? 0 : Math.min(length - 1, activeIndex + 1);
  if (key === "ArrowUp") return activeIndex < 0 ? length - 1 : Math.max(0, activeIndex - 1);
  return null;
}

export function Picker<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
}: {
  value: T;
  options: { value: T; label: string; title?: string; badge?: string }[];
  onChange: (v: T) => void;
  ariaLabel?: string;
}) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [at, setAt] = useState<Placement | null>(null);
  const [host, setHost] = useState<Element | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const uid = useId();

  useEffect(() => {
    const click = (e: MouseEvent) => {
      const t = e.target as Node;
      if (containerRef.current?.contains(t) || menuRef.current?.contains(t)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", click);
    return () => document.removeEventListener("mousedown", click);
  }, []);
  useLayoutEffect(() => {
    setHost(portalHost(containerRef.current));
  }, []);

  useEffect(() => {
    if (!open) return;
    const onScroll = (e: Event) => {
      if (!menuRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onResize = () => setOpen(false);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onResize);
    };
  }, [open]);

  useLayoutEffect(() => {
    if (!open || at || !buttonRef.current || !menuRef.current) return;
    const anchor = buttonRef.current.getBoundingClientRect();
    setAt(
      placeStacked(
        anchor,
        anchor.width,
        Math.min(MENU_MAX_H, menuRef.current.scrollHeight),
        window.innerWidth,
        window.innerHeight,
      ),
    );
  }, [open, at]);

  const currentIndex = options.findIndex((o) => o.value === value);
  const currentOption = options.find((o) => o.value === value);
  const currentLabel = currentOption?.label ?? value;

  const openMenu = () => {
    setAt(null);
    setOpen(true);
    setActiveIndex(currentIndex >= 0 ? currentIndex : 0);
  };
  const closeMenu = () => {
    setOpen(false);
    setActiveIndex(-1);
    buttonRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      if (open) {
        e.preventDefault();
        closeMenu();
      }
      return;
    }
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      if (!open) openMenu();
      else if (activeIndex >= 0) {
        onChange(options[activeIndex].value);
        closeMenu();
      }
      return;
    }
    const next = pickerNextIndex(e.key, open ? activeIndex : -1, options.length);
    if (next === null) return;
    e.preventDefault();
    if (!open) openMenu();
    else setActiveIndex(next);
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
        {currentOption?.badge && <span className="e-picker-badge">{currentOption.badge}</span>}
        <motion.span animate={{ rotate: open ? 180 : 0 }} transition={{ duration: 0.15 }}>
          <IconChevronDown size={15} style={{ opacity: 0.6 }} />
        </motion.span>
      </button>

      {host &&
        createPortal(
          <AnimatePresence>
            {open && (
              <motion.div
                ref={menuRef}
                className="e-picker-menu"
                role="listbox"
                aria-label={ariaLabel}
                initial={{ opacity: 0, y: at?.flipped ? 4 : -4, scale: 0.98 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, y: at?.flipped ? 4 : -4, scale: 0.98 }}
                transition={{ duration: 0.12 }}
                style={{
                  position: "fixed",
                  left: at?.left ?? 0,
                  top: at?.top ?? 0,
                  width: buttonRef.current?.offsetWidth,
                  visibility: at ? "visible" : "hidden",
                  padding: 4,
                  maxHeight: MENU_MAX_H,
                  overflowY: "auto",
                  boxSizing: "border-box",
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
                    onClick={() => {
                      onChange(opt.value);
                      closeMenu();
                    }}
                    className={`e-picker-opt${value === opt.value ? " on" : ""}`}
                    style={{
                      outline: i === activeIndex ? "2px solid var(--e-focus)" : "none",
                      outlineOffset: -2,
                    }}
                  >
                    {opt.label}
                    {opt.badge && <span className="e-picker-badge">{opt.badge}</span>}
                  </button>
                ))}
              </motion.div>
            )}
          </AnimatePresence>,
          host,
        )}
    </div>
  );
}
