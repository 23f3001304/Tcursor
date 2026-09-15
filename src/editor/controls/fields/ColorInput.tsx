export function toHex(c: [number, number, number]): string {
  return (
    "#" +
    c
      .map((v) =>
        Math.max(0, Math.min(255, Math.round(v)))
          .toString(16)
          .padStart(2, "0"),
      )
      .join("")
  );
}

export function fromHex(hex: string): [number, number, number] {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return [0, 0, 0];
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function ColorInput({
  value,
  onChange,
  ariaLabel,
  label,
}: {
  value: [number, number, number];
  onChange: (c: [number, number, number]) => void;
  ariaLabel: string;
  label?: string;
}) {
  return (
    <label className="e-colorpick" title={ariaLabel}>
      <input
        type="color"
        value={toHex(value)}
        aria-label={ariaLabel}
        onChange={(e) => onChange(fromHex(e.target.value))}
      />
      {label && <span>{label}</span>}
    </label>
  );
}
