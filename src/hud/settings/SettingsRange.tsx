import { useRef } from "react";
import { motion } from "motion/react";

function Slider({
  value,
  min,
  max,
  step = 0.01,
  onChange,
  accentColor = "var(--accent, #ef4444)",
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  accentColor?: string;
}) {
  const trackRef = useRef<HTMLDivElement>(null);

  const handlePointerDown = (e: React.PointerEvent) => {
    if (!trackRef.current) return;
    trackRef.current.setPointerCapture(e.pointerId);
    updateValue(e.clientX);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!trackRef.current || !trackRef.current.hasPointerCapture(e.pointerId)) return;
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
    const rawVal = min + pct * (max - min);
    const stepsCount = Math.round((rawVal - min) / step);
    const steppedVal = Math.max(min, Math.min(max, min + stepsCount * step));
    const stepStr = step.toString();
    const decimalIndex = stepStr.indexOf(".");
    const precision = decimalIndex === -1 ? 0 : stepStr.length - decimalIndex - 1;
    onChange(Number(steppedVal.toFixed(precision)));
  };

  const pct = Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100));

  return (
    <div
      ref={trackRef}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      style={{
        position: "relative",
        height: 20,
        display: "flex",
        alignItems: "center",
        cursor: "pointer",
        userSelect: "none",
        width: "100%",
        touchAction: "none",
      }}
    >
      <div
        style={{
          width: "100%",
          height: 6,
          borderRadius: 999,
          background: "var(--hover, #f4f4f5)",
          border: "1px solid var(--line, rgba(0, 0, 0, 0.08))",
          position: "relative",
        }}
      >
        <div
          style={{
            position: "absolute",
            left: 0,
            top: -1,
            bottom: -1,
            width: `${pct}%`,
            background: accentColor,
            borderRadius: 999,
          }}
        />
        <motion.div
          whileHover={{ scale: 1.25 }}
          whileTap={{ scale: 0.95 }}
          transition={{ type: "spring", stiffness: 400, damping: 25 }}
          style={{
            position: "absolute",
            left: `calc(${pct}% - 8px)`,
            top: -5,
            width: 14,
            height: 14,
            borderRadius: "50%",
            background: "#fff",
            border: `2px solid ${accentColor}`,
            boxShadow: "0 2px 4px rgba(0,0,0,0.2)",
          }}
        />
      </div>
    </div>
  );
}

export function Range({
  label,
  value,
  min,
  max,
  step,
  onChange,
  fmt,
  hint,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
  fmt: (v: number) => string;
  hint?: string;
}) {
  return (
    <label className="rng">
      <span className="rng-head">
        <span className="rng-label">{label}</span>
        <span style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
          {hint && <span className="sf-hint">{hint}</span>}
          <span className="rng-val">{fmt(value)}</span>
        </span>
      </span>
      <Slider min={min} max={max} step={step} value={value} onChange={onChange} />
    </label>
  );
}
