import { useRef } from "react";
import { motion } from "motion/react";

/** Snap a raw value to the nearest `step`, clamped to `[min, max]`, with output precision
 *  matching `step`'s own decimal places (so e.g. `step=0.01` never produces
 *  `0.30000000000000004`). Shared by the pointer-drag and keyboard input paths below. */
export function snapToStep(raw: number, min: number, max: number, step: number): number {
  const stepsCount = Math.round((raw - min) / step);
  const stepped = Math.max(min, Math.min(max, min + stepsCount * step));
  const stepStr = step.toString();
  const decimalIndex = stepStr.indexOf(".");
  const precision = decimalIndex === -1 ? 0 : stepStr.length - decimalIndex - 1;
  return Number(stepped.toFixed(precision));
}

/** The value a key press should move a slider to, or `null` if the key isn't one of the
 *  standard slider keys (WAI-ARIA slider pattern): arrows move by one `step`, PageUp/PageDown by
 *  10 steps, Home/End jump to the bounds. */
export function sliderKeyValue(key: string, value: number, min: number, max: number, step: number): number | null {
  switch (key) {
    case "ArrowRight": case "ArrowUp": return snapToStep(value + step, min, max, step);
    case "ArrowLeft": case "ArrowDown": return snapToStep(value - step, min, max, step);
    case "PageUp": return snapToStep(value + step * 10, min, max, step);
    case "PageDown": return snapToStep(value - step * 10, min, max, step);
    case "Home": return min;
    case "End": return max;
    default: return null;
  }
}

// 4. Custom range Slider component with spring animations
export function Slider({
  value,
  min,
  max,
  step = 0.01,
  onChange,
  disabled = false,
  accentColor = "var(--e-fg)",
  ariaLabel,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  disabled?: boolean;
  accentColor?: string;
  ariaLabel?: string;
}) {
  const trackRef = useRef<HTMLDivElement>(null);

  const handlePointerDown = (e: React.PointerEvent) => {
    if (disabled || !trackRef.current) return;
    trackRef.current.setPointerCapture(e.pointerId);
    updateValue(e.clientX);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (disabled || !trackRef.current || !trackRef.current.hasPointerCapture(e.pointerId)) return;
    updateValue(e.clientX);
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    if (trackRef.current && trackRef.current.hasPointerCapture(e.pointerId)) {
      trackRef.current.releasePointerCapture(e.pointerId);
    }
  };

  const updateValue = (clientX: number) => {
    if (!trackRef.current) return;
    const rect = trackRef.current.getBoundingClientRect();
    const pct = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    onChange(snapToStep(min + pct * (max - min), min, max, step));
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (disabled) return;
    const next = sliderKeyValue(e.key, value, min, max, step);
    if (next === null) return;
    e.preventDefault();
    onChange(next);
  };

  const pct = Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100));

  return (
    <div
      ref={trackRef}
      className="e-slider-track"
      role="slider"
      aria-label={ariaLabel}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-disabled={disabled || undefined}
      tabIndex={disabled ? -1 : 0}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onKeyDown={handleKeyDown}
      style={{
        position: "relative",
        height: 20,
        display: "flex",
        alignItems: "center",
        cursor: disabled ? "default" : "pointer",
        userSelect: "none",
        width: "100%",
        touchAction: "none",
        opacity: disabled ? 0.5 : 1
      }}
    >
      {/* Background Track */}
      <div
        style={{
          width: "100%",
          height: 4,
          borderRadius: 999,
          background: "var(--e-soft)",
          border: "1px solid var(--e-border2)",
          position: "relative"
        }}
      >
        {/* Fill Track */}
        <div
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            bottom: 0,
            width: `${pct}%`,
            background: accentColor,
            borderRadius: 999
          }}
        />
        {/* Thumb */}
        <motion.div
          whileHover={disabled ? {} : { scale: 1.25 }}
          whileTap={disabled ? {} : { scale: 0.95 }}
          transition={{ type: "tween", duration: 0.12, ease: [0.4, 0, 0.2, 1] }}
          style={{
            position: "absolute",
            left: `calc(${pct}% - 7px)`,
            top: -6,
            width: 14,
            height: 14,
            borderRadius: "50%",
            background: "#fff",
            border: `2px solid ${disabled ? "var(--e-dim)" : accentColor}`,
            boxShadow: "0 2px 4px rgba(0,0,0,0.3)"
          }}
        />
      </div>
    </div>
  );
}
