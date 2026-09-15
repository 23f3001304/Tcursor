import { useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import { debounce } from "../../util/debounce";
import { shouldClearOverride } from "../../util/overrideClear";
import { SliderValue } from "./SliderValue";

const COMMIT_DEBOUNCE_MS = 80;

const THUMB = 14;

export function snapToStep(raw: number, min: number, max: number, step: number): number {
  const stepsCount = Math.round((raw - min) / step);
  const stepped = Math.max(min, Math.min(max, min + stepsCount * step));
  const stepStr = step.toString();
  const decimalIndex = stepStr.indexOf(".");
  const precision = decimalIndex === -1 ? 0 : stepStr.length - decimalIndex - 1;
  return Number(stepped.toFixed(precision));
}

export function sliderKeyValue(
  key: string,
  value: number,
  min: number,
  max: number,
  step: number,
): number | null {
  switch (key) {
    case "ArrowRight":
    case "ArrowUp":
      return snapToStep(value + step, min, max, step);
    case "ArrowLeft":
    case "ArrowDown":
      return snapToStep(value - step, min, max, step);
    case "PageUp":
      return snapToStep(value + step * 10, min, max, step);
    case "PageDown":
      return snapToStep(value - step * 10, min, max, step);
    case "Home":
      return min;
    case "End":
      return max;
    default:
      return null;
  }
}

export function Slider({
  value,
  min,
  max,
  step = 0.01,
  onChange,
  disabled = false,
  accentColor = "var(--e-fg)",
  ariaLabel,
  label,
  formatValue,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  disabled?: boolean;
  accentColor?: string;
  ariaLabel?: string;
  label?: string;
  formatValue?: (v: number) => string;
}) {
  const trackRef = useRef<HTMLDivElement>(null);
  const [dragValue, setDragValue] = useState<number | null>(null);
  const [dragging, setDragging] = useState(false);
  const settledValueRef = useRef(value);

  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const debouncedRef = useRef<ReturnType<typeof debounce<[number]>> | null>(null);
  if (!debouncedRef.current)
    debouncedRef.current = debounce((v: number) => onChangeRef.current(v), COMMIT_DEBOUNCE_MS);

  useEffect(() => {
    if (shouldClearOverride(dragging, dragValue !== null, value, settledValueRef.current)) setDragValue(null);
  }, [value, dragging, dragValue]);
  useEffect(() => () => debouncedRef.current?.flush(), []);

  const updateValue = (clientX: number) => {
    if (!trackRef.current) return;
    const rect = trackRef.current.getBoundingClientRect();
    const pct = Math.max(0, Math.min(1, (clientX - rect.left - THUMB / 2) / Math.max(1, rect.width - THUMB)));
    const v = snapToStep(min + pct * (max - min), min, max, step);
    setDragValue(v);
    debouncedRef.current!(v);
  };

  const handlePointerDown = (e: React.PointerEvent) => {
    if (disabled || !trackRef.current) return;
    trackRef.current.setPointerCapture(e.pointerId);
    setDragging(true);
    settledValueRef.current = value;
    updateValue(e.clientX);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (disabled || !trackRef.current || !trackRef.current.hasPointerCapture(e.pointerId)) return;
    updateValue(e.clientX);
  };

  const endDrag = (e: React.PointerEvent) => {
    if (trackRef.current?.hasPointerCapture(e.pointerId)) trackRef.current.releasePointerCapture(e.pointerId);
    setDragging(false);
    debouncedRef.current!.flush();
  };

  const commitNow = (next: number) => {
    settledValueRef.current = value;
    setDragValue(next);
    debouncedRef.current!.cancel();
    onChangeRef.current(next);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (disabled) return;
    const next = sliderKeyValue(e.key, shown, min, max, step);
    if (next === null) return;
    e.preventDefault();
    commitNow(next);
  };

  const shown = dragValue ?? value;
  const pct = Math.max(0, Math.min(100, ((shown - min) / (max - min)) * 100));

  return (
    <>
      {label && (
        <SliderValue
          label={label}
          value={shown}
          text={formatValue ? formatValue(shown) : String(shown)}
          min={min}
          max={max}
          step={step}
          disabled={disabled}
          onCommit={commitNow}
        />
      )}
      <div
        ref={trackRef}
        className="e-slider-track"
        role="slider"
        aria-label={ariaLabel}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={shown}
        aria-disabled={disabled || undefined}
        tabIndex={disabled ? -1 : 0}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onLostPointerCapture={endDrag}
        onKeyDown={handleKeyDown}
        style={{
          position: "relative",
          height: 24,
          display: "flex",
          alignItems: "center",
          cursor: disabled ? "default" : "pointer",
          userSelect: "none",
          width: "100%",
          touchAction: "none",
          opacity: disabled ? 0.5 : 1,
        }}
      >
        <div className="e-slider-rail">
          <div className="e-slider-fill" style={{ width: `${pct}%`, background: accentColor }} />
          <motion.div
            className="e-slider-thumb"
            whileHover={disabled ? {} : { scale: 16 / 14 }}
            whileTap={disabled ? {} : { scale: 0.92 }}
            transition={{ type: "spring", stiffness: 500, damping: 30 }}
            style={{ left: `${pct}%`, borderColor: disabled ? "var(--e-dim)" : undefined }}
          />
        </div>
      </div>
    </>
  );
}
