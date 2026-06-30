# src/hud/SettingsAppearance.tsx

Settings panel for configuring frame appearance per recording mode. It keeps a local mode-tab selection and rewrites the correct `ModeAppearance` slice inside `AppearanceSettings` on every field change - the parent holds actual state.

## SettingsAppearance

```tsx
export function SettingsAppearance({
  value,
  onChange,
}: {
  value: AppearanceSettings;
  onChange: (v: AppearanceSettings) => void;
})
```

Edits the `appearance` group of `Settings` - specifically the per-mode `ModeAppearance` slices inside `AppearanceSettings`.

### Props

- `value: AppearanceSettings` - current appearance state; one `ModeAppearance` entry per `ModeKey` (`screen`, `camera`, `presenter`, `screen_only`, `camera_only`). *Why:* the panel is a pure controlled component; the parent (Preferences) holds the single source of truth and persists it.
- `onChange: (v: AppearanceSettings) => void` - receives a fully replaced `AppearanceSettings` on every edit. *Why:* immutable replacement keeps undo and persistence simple - callers compare objects rather than field diffs.

### Behavior

Renders `<section className="sec">` with heading "Frame appearance". Controls in source order:

1. **Mode pill group** (`Pills`) - one pill per entry in `MODES` from `appearanceFields` (Screen / Camera / Presenter / S-only / C-only). Clicking a pill sets the local `mode` state only; it does not write to `value`. *Why:* the mode selector is UI navigation, not a stored preference - appearance settings exist for all modes simultaneously.

2. **`LayoutPreview`** - receives the current `mode` and the active `ModeAppearance` slice `ma = value[mode]`. Redraws live as sliders change.

3. **Range sliders** - one per key in `MODE_SLIDERS[mode]` (resolved against the `SLIDERS` spec map). Each slider writes `value[mode][k]` via the typed `set` helper. Values display via the `pct` formatter (e.g. `"32%"`). *Why per-mode:* some modes lack a webcam, so their slider set is smaller - `screen_only` omits all camera-sizing knobs.

4. **Webcam shape** (`Seg` with `SHAPES`) - shown when `MODE_HAS_SHAPE[mode]` is true. Writes `cam_shape` (circle / rounded / rect). When `cam_shape === "rounded"`, a conditional `Range` for `cam_radius` appears below it. *Why conditional:* `cam_radius` is meaningless for circle and rect, so hiding it avoids confusion.

5. **Webcam corner** (`Seg` with `CORNERS`) - shown when `MODE_HAS_CORNER[mode]` is true (screen mode only). Writes `cam_corner` (BL / BR / TL / TR). *Why gated:* only PiP-style modes where the webcam floats in a corner expose this control.

6. **Reset to defaults** - calls `onChange(DEFAULT_APPEARANCE)`. Restores all five modes at once. *Why all modes:* partial resets would leave other-mode settings inconsistent with the defined defaults.

### Notes

- `Pills` and `Seg` are file-private generic helpers. `Pills` reuses `.optg`/`.opt` CSS classes shared with the FX tab; `Seg` uses `.seg`/`.seg-btn`.
- The `set` helper is a typed shorthand: `set(k, v)` produces `onChange({ ...value, [mode]: { ...ma, [k]: v } })`, always emitting a fresh top-level object and leaving all other modes untouched.
- `cam_margin_x` / `cam_margin_y` sliders appear only in `screen` and `camera` modes per the `MODE_SLIDERS` matrix; they are absent in `camera_only` and `presenter` where the webcam fills the frame.
