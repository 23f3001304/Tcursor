import { useEffect, useRef, useState } from "react";
import { motion } from "motion/react";
import { debounce } from "../hooks/debounce";
import { shouldClearOverride } from "../hooks/overrideClear";
import { SliderValue } from "./SliderValue";

// Trailing debounce window for committing a drag to the caller's `onChange` (which is typically
// an `apply_edit_op`/`save_edit` IPC round trip) - see editor.md "render hygiene". A pointer
// release always `flush()`es immediately regardless of this window, so letting go never lags.
const COMMIT_DEBOUNCE_MS = 80;

/** Snap a raw value to the nearest `step`, clamped to `[min, max]`, with output precision
 *  matching `step`'s own decimal places (so e.g. `step=0.01` never produces
 *  `0.30000000000000004`). Shared by the pointer-drag and keyboard input paths below. */
/** `.e-slider-thumb`'s width (controls.css), which insets the rail by half on each side. */
const THUMB = 14;

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
  /** Optional `.e-fl` row above the track: the name on the left, the value on the right, where
   *  the value is click-to-type (`SliderValue`). Sourced from the LIVE `shown` value (not the
   *  committed `value` prop) so it tracks the thumb during a drag. Omit to render nothing (the
   *  caller keeps rendering its own label from `value`, as every call site used to). */
  label?: string;
  /** Formats `shown` for `label` above, e.g. `(v) => \`${Math.round(v * 100)}%\`` - falls back to
   *  the raw number when `label` is given without this. */
  formatValue?: (v: number) => string;
}) {
  const trackRef = useRef<HTMLDivElement>(null);
  // The live, pointer/keypress-derived value - drawn instead of `value` (and fed to `label`'s
  // readout) so both track input at full rate despite the debounced `onChange` commit below (the
  // "optimistic local UI" half of the render-hygiene fix). `null` = not overriding.
  const [dragValue, setDragValue] = useState<number | null>(null);
  const [dragging, setDragging] = useState(false);
  // `value` as of the moment the CURRENT gesture began - `shouldClearOverride` (`../hooks/
  // overrideClear.ts`) clears the override on the FIRST value the prop takes on that differs from
  // this snapshot, not on landing back on the exact value sent - see that file for why.
  const settledValueRef = useRef(value);

  // `onChange` read through a ref so the debounced wrapper below never needs recreating just
  // because the caller passed a fresh inline arrow (as most do).
  const onChangeRef = useRef(onChange); onChangeRef.current = onChange;
  const debouncedRef = useRef<ReturnType<typeof debounce<[number]>> | null>(null);
  if (!debouncedRef.current) debouncedRef.current = debounce((v: number) => onChangeRef.current(v), COMMIT_DEBOUNCE_MS);

  useEffect(() => {
    if (shouldClearOverride(dragging, dragValue !== null, value, settledValueRef.current)) setDragValue(null);
  }, [value, dragging, dragValue]);
  // A pending commit must still land even if the slider unmounts mid-drag (switching panels,
  // deselecting) - flush rather than drop it.
  useEffect(() => () => debouncedRef.current?.flush(), []);

  const updateValue = (clientX: number) => {
    if (!trackRef.current) return;
    const rect = trackRef.current.getBoundingClientRect();
    // The rail is inset by half a thumb on each side (`.e-slider-rail`), so the pointer maps onto
    // the rail's own span, not the full strip: a press on the strip's first pixel is 0.
    const pct = Math.max(0, Math.min(1, (clientX - rect.left - THUMB / 2) / Math.max(1, rect.width - THUMB)));
    const v = snapToStep(min + pct * (max - min), min, max, step);
    setDragValue(v);
    debouncedRef.current!(v);
  };

  const handlePointerDown = (e: React.PointerEvent) => {
    if (disabled || !trackRef.current) return;
    trackRef.current.setPointerCapture(e.pointerId);
    setDragging(true);
    settledValueRef.current = value; // the pre-drag value - the override clears once `value` moves off of it
    updateValue(e.clientX);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (disabled || !trackRef.current || !trackRef.current.hasPointerCapture(e.pointerId)) return;
    updateValue(e.clientX);
  };

  // Shared end-of-gesture path: `pointerup`, `pointercancel` (palm rejection, a system gesture
  // stealing the pointer), and `lostpointercapture` all mean "the drag is over" - wiring all
  // three matters because a cancelled/lost-capture sequence never fires `pointerup`, which would
  // otherwise leave `dragging` stuck `true` forever and permanently block the clear-on-change
  // effect above. Safe to run twice per gesture (`lostpointercapture` also follows a normal
  // release's own capture release) - every step here is idempotent.
  const endDrag = (e: React.PointerEvent) => {
    if (trackRef.current?.hasPointerCapture(e.pointerId)) trackRef.current.releasePointerCapture(e.pointerId);
    setDragging(false);
    debouncedRef.current!.flush(); // commit on release - no trailing lag survives the drag ending
  };

  // Shared by the keyboard path and the typed readout: both are discrete, so both bypass the
  // drag debounce entirely (there is nothing to coalesce) while still painting optimistically.
  const commitNow = (next: number) => {
    settledValueRef.current = value; // pre-press value - see the ref's own comment above
    setDragValue(next);
    debouncedRef.current!.cancel();
    onChangeRef.current(next);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (disabled) return;
    // Steps from `shown` (not the prop `value`) so a fast key-repeat before an async `onChange`
    // round trip resolves still steps from wherever the last keypress optimistically landed.
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
        <SliderValue label={label} value={shown} text={formatValue ? formatValue(shown) : String(shown)}
          min={min} max={max} step={step} disabled={disabled} onCommit={commitNow} />
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
        // 24px, not the 20 it used to be: this strip IS the control's hit target (the 4px rail is
        // only paint), and 24 is the floor the usability pass set for anything clickable.
        style={{ position: "relative", height: 24, display: "flex", alignItems: "center",
          cursor: disabled ? "default" : "pointer", userSelect: "none", width: "100%",
          touchAction: "none", opacity: disabled ? 0.5 : 1 }}
      >
        {/* Rail (flat groove) + fill */}
        <div className="e-slider-rail">
          <div className="e-slider-fill" style={{ width: `${pct}%`, background: accentColor }} />
          {/* Thumb - the raised interactive nub; hover grows it 14->16 via a `scale` transform
              (not width/height), so its fixed margin-based centering never needs to recompute. */}
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
