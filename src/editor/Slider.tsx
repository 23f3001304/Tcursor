import { useRef } from "react";
import { motion } from "motion/react";

// 4. Custom range Slider component with spring animations
export function Slider({
  value,
  min,
  max,
  step = 0.01,
  onChange,
  disabled = false,
  accentColor = "var(--e-fg)"
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (v: number) => void;
  disabled?: boolean;
  accentColor?: string;
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
    const rawVal = min + pct * (max - min);

    // Calculate nearest step
    const stepsCount = Math.round((rawVal - min) / step);
    const steppedVal = Math.max(min, Math.min(max, min + stepsCount * step));

    // Determine precision based on step
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
          transition={{ type: "spring", stiffness: 400, damping: 25 }}
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
