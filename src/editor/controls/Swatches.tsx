// A grid/row of clickable color (or gradient) swatches with a selection ring - the one shared
// component behind every "pick a preset" control in the editor: BackgroundPanel's color/gradient/
// accent grids, EffectsPanel's ripple-color/tint rows, CameraRingField's ring-color row. Each call
// site keeps its own preset data and selection logic (colors, gradients, and even the accent list
// are shaped differently); this component only owns rendering the buttons + the ring/border.
export type SwatchVariant = "preset" | "accent" | "small";

/** One swatch: `css` is what actually paints the button's `background` (a solid `rgb(...)` or a
 *  `linear-gradient(...)` string); `value` is the caller's own data for that swatch, handed back
 *  to `isSelected`/`onSelect` unchanged so callers never have to re-derive it from `css`. */
export interface SwatchItem<T> {
  key: string;
  css: string;
  value: T;
  ariaLabel?: string;
}

const WRAP_CLASS: Record<SwatchVariant, string> = {
  preset: "e-preset-grid",
  accent: "e-accent-list",
  small: "e-swatch-row",
};
const BTN_CLASS: Record<SwatchVariant, string> = {
  preset: "e-preset-circle",
  accent: "e-accent-circle",
  small: "e-swatch-sm",
};

export function Swatches<T>({
  items,
  isSelected,
  onSelect,
  variant = "small",
  disabled,
}: {
  items: SwatchItem<T>[];
  isSelected: (value: T) => boolean;
  onSelect: (value: T) => void;
  variant?: SwatchVariant;
  disabled?: boolean;
}) {
  return (
    <div className={WRAP_CLASS[variant]}>
      {items.map((item) => (
        <button
          key={item.key}
          type="button"
          disabled={disabled}
          // A swatch has no text, so it needs both: the name for a screen reader and the same
          // name on hover for everyone else (benchmark tell 5 - no unlabelled icon-only controls).
          title={item.ariaLabel ?? item.key}
          aria-label={item.ariaLabel ?? item.key}
          className={`${BTN_CLASS[variant]} ${isSelected(item.value) ? "on" : ""}`}
          style={{ background: item.css }}
          onClick={() => onSelect(item.value)}
        />
      ))}
    </div>
  );
}
