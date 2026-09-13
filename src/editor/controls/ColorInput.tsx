// A native `<input type="color">` dressed as one of the panel's swatches - the custom half of
// every "pick a preset OR pick your own" control (BackgroundPanel's Color and Gradient tabs). The
// OS owns the picker popover; this component owns only the trigger's look and the RGB <-> hex
// conversion, so a colour chosen here is the exact `[r, g, b]` shape settings already store.

/** `[r, g, b]` -> `#rrggbb`, the only form `<input type="color">` accepts. */
export function toHex(c: [number, number, number]): string {
  return "#" + c.map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0")).join("");
}

/** `#rrggbb` -> `[r, g, b]`. Anything else returns black rather than throwing - the input element
 *  only ever emits the canonical form, so this is a type guard, not a parser. */
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
  /** Optional caption under the swatch (the gradient tab labels its From/Mid/To stops). */
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
