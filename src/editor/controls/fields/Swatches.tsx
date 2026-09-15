export type SwatchVariant = "preset" | "accent" | "small";

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
