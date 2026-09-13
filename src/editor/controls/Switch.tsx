import { motion, useReducedMotion } from "motion/react";

// The panel pass moved this switch's paint out of inline styles and into `.sw` (controls.css):
// the track is a plane (raised when off, the app's one accent when on) with no border at all, so
// an inline `background` can no longer beat the stylesheet's own hover state. What stays inline is
// the thumb's position, which Motion animates.
//
// `disabled` (T34 L3): a switch whose OFF state the backend would reject must read as unavailable,
// not silently snap back - see LayoutInspector's panel-visibility rows.
export function Switch({ on, onChange, ariaLabel, disabled, title }: { on: boolean; onChange: (v: boolean) => void; ariaLabel?: string; disabled?: boolean; title?: string }) {
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
