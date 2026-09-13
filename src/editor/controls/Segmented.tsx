import { useId, useRef } from "react";
import { motion, useReducedMotion } from "motion/react";

/** The option index an arrow/Home/End key should move a segmented control's selection to, or
 *  `null` for any other key. Wraps at both ends (the WAI-ARIA radio-group convention, unlike
 *  `pickerNextIndex`'s clamped listbox), because a 2-to-4 segment row is short enough that
 *  wrapping is faster than reversing direction. */
export function segmentedNextIndex(key: string, index: number, length: number): number | null {
  if (length === 0) return null;
  const at = index < 0 ? 0 : index;
  switch (key) {
    case "ArrowRight": case "ArrowDown": return (at + 1) % length;
    case "ArrowLeft": case "ArrowUp": return (at - 1 + length) % length;
    case "Home": return 0;
    case "End": return length - 1;
    default: return null;
  }
}

/** A row of exclusive choices, one visible press away each. Same `value`/`options`/`onChange`
 *  shape as `Picker`, so a 2-to-4 option dropdown becomes one by swapping the component name -
 *  see `docs/api/src/editor/controls/Segmented.md` for which controls qualify. */
export function Segmented<T extends string>({ value, options, onChange, ariaLabel, columns }: {
  value: T;
  options: { value: T; label: string; title?: string }[];
  onChange: (v: T) => void;
  ariaLabel?: string;
  /** Wrap into a grid of this many columns instead of one row - for four short labels that read
   *  better 2x2 than squeezed four-up at 320px (CameraPanel's Dock Location). */
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
    <div ref={wrap} className="e-segmented" role="radiogroup" aria-label={ariaLabel}
      style={columns ? { display: "grid", gridTemplateColumns: `repeat(${columns}, 1fr)` } : undefined}
      onKeyDown={onKeyDown}>
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button key={o.value} type="button" role="radio" aria-checked={on} title={o.title ?? o.label}
            tabIndex={on || (index < 0 && o === options[0]) ? 0 : -1}
            className={`e-segment${on ? " on" : ""}`} onClick={() => onChange(o.value)}>
            {on && (
              <motion.span className="e-segment-slot" layoutId={still ? undefined : `${uid}-slot`}
                transition={{ type: "spring", stiffness: 500, damping: 30 }} />
            )}
            <span className="e-segment-label">{o.label}</span>
          </button>
        );
      })}
    </div>
  );
}
