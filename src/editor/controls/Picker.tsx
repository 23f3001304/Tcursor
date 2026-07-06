import { useState, useRef, useEffect } from "react";
import { motion, AnimatePresence } from "motion/react";
import { IconChevronDown } from "@tabler/icons-react";

// 2. Custom dropdown select Picker component
export function Picker<T extends string>({
  value,
  options,
  onChange
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const click = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", click);
    return () => document.removeEventListener("mousedown", click);
  }, []);

  const currentLabel = options.find((o) => o.value === value)?.label ?? value;

  return (
    <div ref={containerRef} style={{ position: "relative", width: "100%" }}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
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
            {options.map((opt) => (
              <button
                key={opt.value}
                type="button"
                onClick={() => {
                  onChange(opt.value);
                  setOpen(false);
                }}
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
