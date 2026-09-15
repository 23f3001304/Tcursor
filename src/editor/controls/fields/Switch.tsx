import { motion, useReducedMotion } from "motion/react";

export function Switch({
  on,
  onChange,
  ariaLabel,
  disabled,
  title,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  ariaLabel?: string;
  disabled?: boolean;
  title?: string;
}) {
  const still = useReducedMotion();
  return (
    <button
      type="button"
      className={`sw ${on ? "on" : ""}`}
      role="switch"
      aria-checked={on}
      aria-label={ariaLabel}
      title={title}
      disabled={disabled}
      onClick={() => onChange(!on)}
    >
      <motion.span
        className="sw-thumb"
        layout={!still}
        transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
        style={{ left: on ? 20 : 3 }}
      />
    </button>
  );
}
