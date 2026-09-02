import { motion } from "motion/react";

// 1. iOS-style Switch component with spring animation
export function Switch({ on, onChange, ariaLabel }: { on: boolean; onChange: (v: boolean) => void; ariaLabel?: string }) {
  return (
    <button
      type="button"
      className={`sw ${on ? "on" : ""}`}
      role="switch"
      aria-checked={on}
      aria-label={ariaLabel}
      onClick={() => onChange(!on)}
      style={{
        width: 38,
        height: 22,
        borderRadius: 999,
        background: on ? "var(--e-fg)" : "var(--e-soft)",
        border: `1px solid ${on ? "var(--e-fg)" : "var(--e-border)"}`,
        position: "relative",
        cursor: "pointer",
        padding: 0,
        display: "inline-flex",
        alignItems: "center",
        transition: "background-color 0.14s ease, border-color 0.14s ease",
        outline: "none"
      }}
    >
      <motion.span
        layout
        transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
        style={{
          width: 16,
          height: 16,
          borderRadius: "50%",
          // Neutral switch: on = zinc-white track with a near-black thumb; off = a muted
          // (--e-dim) thumb on the soft track, so the off state reads clearly muted rather
          // than as a bright floating dot.
          background: on ? "#09090b" : "var(--e-dim)",
          position: "absolute",
          left: on ? 19 : 3,
          boxShadow: "0 1px 2px rgba(0,0,0,0.1)"
        }}
      />
    </button>
  );
}
