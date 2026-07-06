# src/hud/settings/SettingsCursor.tsx

Settings panel for cursor rendering style and enhanced-mode parameters. Shows the style selector at all times and conditionally reveals size, motion-blur, and bounce controls only when "Enhanced" is active.

## SettingsCursor

```tsx
export function SettingsCursor({
  value,
  onChange,
}: {
  value: CursorSettings;
  onChange: (v: CursorSettings) => void;
})
```

Edits the `cursor` group of `Settings`.

### Props

- `value: CursorSettings` - current cursor settings (style, size, motion_blur, click_bounce, bounce_intensity). *Why:* controlled component; the parent (Preferences) owns and persists state.
- `onChange: (v: CursorSettings) => void` - receives a fully replaced `CursorSettings`. *Why:* immutable replacement keeps the parent's persistence path uniform across all settings panels.

### Behavior

Renders `<section className="sec">` with heading "Cursor". Controls in source order:

1. **Style** (`Field` with hint "Enhanced redraws a smooth pointer") - inline `.seg`/`.seg-btn` segmented control with three options derived from `STYLES`: System / Enhanced / Hidden. Writes `value.style` (`CursorStyle`). *Why three modes:* System delegates cursor rendering to the OS (no overhead), Enhanced redraws a smoother pointer overlaid on the recording, Hidden hides the cursor entirely for clean screencasts.

2. The following controls are gated on `value.style === "enhanced"` and are not rendered otherwise:

   - **Size** (Range 0.5-2.5, step 0.05) - scales the enhanced cursor sprite relative to the system cursor size. Writes `value.size`, displayed as a percentage. *Why 0.5-2.5:* sub-0.5 is too small to see; above 2.5 the cursor overlaps nearby content distractingly.
   - **Motion blur** (Range 0-1, step 0.05) - amount of directional blur applied to the cursor during fast movement. Writes `value.motion_blur`, displayed as a percentage. *Why 0 minimum:* completely disabling motion blur is a valid preference for presenters who want a crisp pointer.
   - **Click bounce** (Switch) - when on, the cursor briefly scales down and back on each click. Writes `value.click_bounce`. *Why a toggle:* bounce is a visual flourish; some presenters find it distracting.
   - **Bounce intensity** (Range 0-1, step 0.05) - depth of the bounce scale-down. Visible only when `value.click_bounce` is true. Writes `value.bounce_intensity`, displayed as a percentage. *Why conditional:* the slider is meaningless when click_bounce is off, so hiding it reduces clutter.

### Notes

- The typed `set` shorthand always produces `onChange({ ...value, [k]: v })` - a fresh object - so other fields are never silently reset.
- `STYLES` is a file-local constant: `[["system","System"],["enhanced","Enhanced"],["hidden","Hidden"]]`. The segmented control is built inline (not via `Seg` from `SettingsAppearance`) using the same `.seg`/`.seg-btn` CSS classes.
