import { useId, useRef } from "react";
import { motion, useReducedMotion } from "motion/react";

export function segmentedNextIndex(key: string, index: number, length: number): number | null {
  if (length === 0) return null;
  const at = index < 0 ? 0 : index;
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
      return (at + 1) % length;
    case "ArrowLeft":
    case "ArrowUp":
      return (at - 1 + length) % length;
    case "Home":
      return 0;
    case "End":
      return length - 1;
    default:
      return null;
  }
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
  columns,
}: {
  value: T;
  options: { value: T; label: string; title?: string }[];
  onChange: (v: T) => void;
  ariaLabel?: string;
  columns?: number;
}) {
  const uid = useId();
  const still = useReducedMotion();
  const wrap = useRef<HTMLDivElement>(null);
  const index = options.findIndex((o) => o.value === value);

  const onKeyDown = (e: React.KeyboardEvent) => {
    const next = segmentedNextIndex(e.key, index, options.length);
    if (next === null) return;
    e.preventDefault();
    onChange(options[next].value);
    wrap.current?.querySelectorAll<HTMLButtonElement>("button")[next]?.focus();
  };

  return (
    <div
      ref={wrap}
      className="e-segmented"
      role="radiogroup"
      aria-label={ariaLabel}
      style={columns ? { display: "grid", gridTemplateColumns: `repeat(${columns}, 1fr)` } : undefined}
      onKeyDown={onKeyDown}
    >
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={on}
            title={o.title ?? o.label}
            tabIndex={on || (index < 0 && o === options[0]) ? 0 : -1}
            className={`e-segment${on ? " on" : ""}`}
            onClick={() => onChange(o.value)}
          >
            {on && (
              <motion.span
                className="e-segment-slot"
                layoutId={still ? undefined : `${uid}-slot`}
                transition={{ type: "spring", stiffness: 500, damping: 30 }}
              />
            )}
            <span className="e-segment-label">{o.label}</span>
          </button>
        );
      })}
    </div>
  );
}
