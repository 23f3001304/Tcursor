# src/hud/settings/SettingsRange.tsx

The HUD settings panels' numeric slider. Split out of `SettingsControls.tsx`, which is the small presentational primitives (a labelled row, a switch, a disclosure); this one is a pointer-driven custom control with its own geometry and stepping, and it was most of that file.

Like everything in `SettingsControls`, it is purely presentational: state is owned by the caller, and it calls back with a new value rather than mutating anything.

## Slider

```tsx
function Slider({ value, min, max, step = 0.01, onChange, accentColor = "var(--accent, #ef4444)" }: {
  value: number; min: number; max: number; step?: number;
  onChange: (v: number) => void; accentColor?: string;
}): JSX.Element
```

The track and the thumb. Not exported: `Range` is the whole control anyone should use, and a bare track with no label or read-out is not something a settings panel wants.

Pointer-driven rather than `<input type="range">`: the native control cannot be painted to match the HUD's plane-and-accent language across both themes without fighting the platform's own thumb. It captures the pointer on the track (`setPointerCapture`), so a drag that leaves the 20px row keeps working, and releases on pointer-up.

`updateValue(clientX)` is the whole rule: the x within the track becomes a 0-to-1 fraction, the fraction becomes a raw value in `[min, max]`, and the raw value snaps to the nearest `step`. It then rounds to the step's OWN decimal count (read off `step.toString()`), because the float arithmetic that produced it would otherwise hand `0.30000000000000004` to a caller whose `fmt` shows one decimal.

The thumb grows on hover and presses in on tap (a spring), so a 14px target says it is a target.

## Range

```tsx
export function Range({ label, value, min, max, step, onChange, fmt, hint }: {
  label: string; value: number; min: number; max: number; step: number;
  onChange: (v: number) => void; fmt: (v: number) => string; hint?: string;
}): JSX.Element
```

A compact numeric slider with its own header. Renders `<label className="rng">` containing a `.rng-head` row (label left, optional hint then formatted value right), then the `Slider`.

### Props

- `label: string` - control label; wrapping in `<label>` makes the entire header row a click target for the slider. *Why:* increases the tap area without extra CSS.
- `value: number` - current numeric value (controlled). *Why controlled:* settings state lives in the parent; the slider must always reflect the persisted value.
- `min / max / step: number` - slider bounds and increment. *Why per-call-site:* each setting has different units and valid ranges (e.g., 0.04-0.3 for smoothness vs. 600-5000 for hold_ms).
- `onChange: (v: number) => void` - called with the stepped, rounded value on every pointer move while the track is captured.
- `fmt: (v: number) => string` - display formatter injected by the caller. *Why injected:* different settings need different formats (percent, raw decimal, seconds) - a built-in enum would be fragile and harder to extend.
- `hint?: string` - muted text shown between the label and the formatted value. *Why:* some sliders carry contextual notes (e.g., a unit label) without needing a separate `Field` wrapper.

### Notes

Used in `SettingsZoom`, `SettingsClickFx`, `SettingsCursor`, `SettingsAppearance` and `SettingsPanel`. The `fmt` prop is mandatory - every call site supplies a formatter such as `(v) => ${Math.round(v * 100)}%` or `(v) => ${v.toFixed(1)}x`.
