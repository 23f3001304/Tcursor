# src/hud/settings/SettingsControls.tsx

Shared primitive UI controls used by every settings panel in the HUD. All four exports are purely presentational - state is always owned by the caller, and each component calls back with a new value rather than mutating anything internally.

## Field

```tsx
export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
})
```

A labelled settings row. Renders a `.sf` wrapper containing a `.sf-head` row (label + optional muted hint), then `children` below it.

### Props

- `label: string` - visible label text for the control. *Why:* consistent label placement across all panels without each panel defining its own layout div.
- `hint?: string` - muted secondary note rendered in `.sf-hint` beside the label. *Why:* in-place guidance (e.g., "also used by the hotkey") without requiring a tooltip or a separate description element.
- `children: ReactNode` - the control to render below the label row. *Why:* `Field` is a layout shell only; the caller supplies the actual input so any control type can be used inside it.

### Notes

Used by `SettingsClickFx`, `SettingsCursor`, `SettingsHotkeys`, `SettingsInterface`, `SettingsZoom`, and `SettingsAppearance`. Some callers use `Field` wrapping a `Seg`/`Picker` directly while others use the bare `.sf`/`.sf-row` pattern for Switch rows (which need horizontal alignment, not vertical stacking).

## Switch

```tsx
export function Switch({
  on,
  onChange,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
})
```

An iOS-style boolean toggle rendered as `<button role="switch" aria-checked={on}>` with a `.sw-knob` thumb inside. Applies `.on` to the button when `on` is true.

### Props

- `on: boolean` - current toggle state. *Why a prop, not internal state:* all settings are externally owned; the switch is purely a controlled display of that state.
- `onChange: (v: boolean) => void` - called with `!on` on click. *Why the new value, not an event:* saves every call site from computing `!on` themselves.

### Notes

Used by every settings panel. Not wrapped in a `Field` - callers typically place it inside a `.sf-row` div with a `.sf-label` span to produce a horizontal label + switch layout.

## Range

```tsx
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
})
```

A compact numeric slider. Renders `<label className="rng">` containing a `.rng-head` row (label left, optional hint then formatted value right), then `<input type="range">`.

### Props

- `label: string` - control label; wrapping in `<label>` makes the entire header row a click target for the slider. *Why:* increases the tap area without extra CSS.
- `value: number` - current numeric value (controlled). *Why controlled:* settings state lives in the parent; the slider must always reflect the persisted value.
- `min / max / step: number` - slider bounds and increment. *Why per-call-site:* each setting has different units and valid ranges (e.g., 0.04-0.3 for smoothness vs. 600-5000 for hold_ms).
- `onChange: (v: number) => void` - called with `parseFloat(e.target.value)` on every change event. *Why parseFloat here:* `input.value` is always a string; centralising the conversion keeps callers typed.
- `fmt: (v: number) => string` - display formatter injected by the caller. *Why injected:* different settings need different formats (percent, raw decimal, seconds) - a built-in enum would be fragile and harder to extend.
- `hint?: string` - muted text shown between the label and the formatted value. *Why:* some sliders carry contextual notes (e.g., a unit label) without needing a separate `Field` wrapper.

### Notes

Used in `SettingsZoom`, `SettingsClickFx`, `SettingsCursor`, and `SettingsAppearance`. The `fmt` prop is mandatory - every call site supplies a formatter such as `(v) => ${Math.round(v * 100)}%` or `(v) => ${v.toFixed(1)}x`.

## Advanced

```tsx
export function Advanced({
  children,
  open,
  onToggle,
}: {
  children: ReactNode;
  open?: boolean;
  onToggle?: (v: boolean) => void;
})
```

A collapsible disclosure section labelled "Advanced". Closed by default. Animates height and opacity via Motion `AnimatePresence` (duration 0.18 s, easeOut) so the reveal feels physical rather than abrupt.

### Props

- `children: ReactNode` - content revealed when expanded. *Why:* keeps infrequently changed knobs out of the default view without moving them to a separate page.
- `open?: boolean` - controlled open state; when provided it takes precedence over internal `useState`. *Why optional:* `SettingsZoom` needs to programmatically open the section when the user clicks "Custom"; other panels let it self-manage.
- `onToggle?: (v: boolean) => void` - called with the new boolean when the toggle button is clicked. Required when `open` is provided. *Why:* without an external setter a controlled parent cannot respond to user-initiated close.

### Behavior

- Uncontrolled mode: omit both `open` and `onToggle`. Internal `openS` drives visibility.
- Controlled mode: provide both `open` and `onToggle`. `openProp ?? openS` resolves to `openProp`, and `setOpen` calls `onToggle` instead of `setOpenS`.
- The toggle button carries `aria-expanded={open}` and adds `.open` when expanded (a CSS caret rotation is expected from the stylesheet).
- `AnimatePresence initial={false}` suppresses the expand animation on mount so a pre-open section does not animate in on initial render.

### Notes

Used by `SettingsClickFx` (uncontrolled) and `SettingsZoom` (controlled, with `adv`/`setAdv` state). The `motion/react` import means this file has a runtime dependency on the Motion library.
