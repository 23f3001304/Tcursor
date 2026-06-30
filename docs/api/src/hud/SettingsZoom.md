# src/hud/SettingsZoom.tsx

Settings panel for the auto-zoom feature. Provides a master toggle, a click-count selector, one-click feel presets, smart-zoom switches, and an expandable advanced panel with per-field sliders. The "Custom" feel button opens the advanced panel programmatically.

## SettingsZoom

```tsx
export function SettingsZoom({
  value,
  onChange,
}: {
  value: ZoomSettings;
  onChange: (v: ZoomSettings) => void;
})
```

Edits the `zoom` group of `Settings`.

### Props

- `value: ZoomSettings` - current zoom settings (all nine fields). *Why:* controlled component; the parent (Preferences) owns and persists state.
- `onChange: (v: ZoomSettings) => void` - receives a fully replaced `ZoomSettings`. *Why:* immutable replacement keeps the parent's persistence path uniform; no partial-update logic is needed in the parent.

### Behavior

Renders `<section className="sec">` with heading "Auto-zoom". Holds one piece of local state: `adv: boolean` - whether the `Advanced` collapsible is open. Controls in source order:

1. **Zoom on click** (Switch) - master toggle. Writes `value.enabled`. *Why:* lets users disable auto-zoom entirely without losing their other settings.

2. **Clicks to zoom** (`Field`) - `.seg`/`.seg-btn` with options 1, 2, 3. Writes `value.clicks`. *Why three choices:* 1 zooms on every click (aggressive); 2 requires a double-click (intentional); 3 requires a triple-click (very deliberate). Matches the `clicks_to_trigger` knob in the Rust `autozoom` module.

3. **Feel** (`Field`) - `.seg`/`.seg-btn` with three hard-coded presets plus "Custom":

   | Preset | target_scale | smoothness | hold_ms |
   |---|---|---|---|
   | Subtle | 1.6 | 0.16 | 1800 |
   | Balanced | 2.2 | 0.10 | 2200 |
   | Punchy | 2.8 | 0.07 | 1400 |

   Clicking a preset calls `onChange({ ...value, ...p.v })`, overwriting all three fields at once. The active preset is identified by `activePreset(value)`, which matches all three values simultaneously; if none match, "Custom" is highlighted. Clicking "Custom" sets `adv = true` (opens Advanced) without changing any field value. *Why three fields per preset:* scale, smoothness, and hold time are coupled - a punchy feel needs a higher scale, faster easing, and shorter hold; exposing them as a bundle avoids incoherent combinations.

4. **Smart zoom** sub-header - an inline uppercase label (not a `Field`). Below it:

   - **Smart type** (Switch) - writes `value.smart_hold`. When true, typing activity extends the zoom hold past the last click. *Why:* a user may stop clicking but keep typing in a focused field; the zoom should stay alive through that activity.
   - **Smart follow** (Switch) - writes `value.smart_follow`. *Why separate:* follow (tracking cursor position) and hold extension (timing) are independent behaviors; a presenter may want one but not the other.

5. **Advanced** (controlled, `open={adv}`, `onToggle={setAdv}`) - contains:

   - **Amount** (Range 1.2-4, step 0.1) - writes `value.target_scale`, formatted as `"2.2x"`. *Why 1.2 minimum:* below 1.2 the zoom is visually imperceptible.
   - **Smoothness** (Range 0.04-0.30, step 0.01) - writes `value.smoothness`, formatted as raw decimal. Lower values produce faster, snappier zoom motion; higher values produce slow easing. *Why inverted intuition:* the value is used as a spring coefficient internally, so smaller = faster.
   - **Hold / Idle release** (Range 600-5000 ms, step 100) - writes `value.hold_ms`, formatted as seconds (e.g., `"2.2 s"`). The label changes to "Idle release" when `value.smart_hold` is true, because in smart mode the hold extends dynamically and this value sets the idle threshold instead of a fixed duration.
   - **Shrink camera on zoom** (Switch) - writes `value.camera_shrink`. *Why:* when zoomed in, the webcam overlay may cover content; shrinking it keeps the screen panel visible.
   - **Min camera size** (Range 0.3-1.0, step 0.02) - visible only when `value.camera_shrink` is true. Writes `value.camera_shrink_min`, displayed as a percentage. *Why conditional:* the slider is meaningless when camera_shrink is off.

### Notes

- `activePreset` is a file-local helper that checks whether the current `value` matches any preset by comparing all three preset fields. It returns the preset name or `"Custom"`.
- The `set` typed shorthand always produces `onChange({ ...value, [k]: v })` - a fresh object - so other fields are never silently reset.
- `PRESETS` and `CLICKS` are file-local constants. Changing preset values here must be coordinated with any UI copy that describes the feel options to users.
