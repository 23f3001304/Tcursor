import { IconPlus, IconMinus } from "@tabler/icons-react";

// 3. Custom Number Input component with +/- stepper buttons
export function NumberField({
  value,
  min = 0,
  max,
  step = 0.1,
  onChange
}: {
  value: number;
  min?: number;
  max?: number;
  step?: number;
  onChange: (v: number) => void;
}) {
  const increment = () => {
    const nv = +(value + step).toFixed(2);
    if (max === undefined || nv <= max) onChange(nv);
  };
  const decrement = () => {
    const nv = +(value - step).toFixed(2);
    if (nv >= min) onChange(nv);
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        background: "var(--e-soft)",
        border: "1px solid var(--e-border)",
        borderRadius: "var(--e-r)",
        height: 36,
        padding: "0 4px",
        width: "100%",
        boxSizing: "border-box"
      }}
    >
      <button
        type="button"
        onClick={decrement}
        disabled={value <= min}
        style={{
          width: 28,
          height: 28,
          borderRadius: "var(--e-r-sm)",
          border: "none",
          background: "transparent",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          cursor: value <= min ? "default" : "pointer",
          color: "var(--e-fg)",
          opacity: value <= min ? 0.3 : 0.6,
          outline: "none"
        }}
      >
        <IconMinus size={13} />
      </button>

      <span
        style={{
          flex: 1,
          minWidth: 0,
          textAlign: "center",
          color: "var(--e-fg)",
          fontSize: 13,
          fontWeight: 600,
          fontVariantNumeric: "tabular-nums",
          userSelect: "none"
        }}
      >
        {value}
        <span style={{ color: "var(--e-mut)", fontWeight: 500, marginLeft: 2 }}>s</span>
      </span>

      <button
        type="button"
        onClick={increment}
        disabled={max !== undefined && value >= max}
        style={{
          width: 28,
          height: 28,
          borderRadius: "var(--e-r-sm)",
          border: "none",
          background: "transparent",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          cursor: max !== undefined && value >= max ? "default" : "pointer",
          color: "var(--e-fg)",
          opacity: max !== undefined && value >= max ? 0.3 : 0.6,
          outline: "none"
        }}
      >
        <IconPlus size={13} />
      </button>
    </div>
  );
}
