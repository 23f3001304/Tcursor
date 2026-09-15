# src/editor/shell/settings/MotionSection.tsx

`EditorSettingsDialog`'s "Motion" section (M3): the project's ONE motion language. Sits between Zoom defaults and Screen, and owns three things - the preset row that writes `settings.motion`, the "Apply to all regions" button, and the Camera smoothing knob moved here from `ZoomDefaultsSection`.

**Why the smoothing knob moved.** `camera_smoothing_ms` is not a seed a new zoom inherits (which is what every other control in Zoom defaults is): it is the global post-pass on the camera's PATH (`export/camera/smoothing.rs`), applied once to the whole track regardless of which region produced it. That is motion, so it belongs here. The FIELD did not move - it still lives on `ZoomSettings`, which is why `DEFAULT_ZOOM_SETTINGS` still carries it and the dialog writes it back through `setZoom`.

Per-region smoothing stays out of scope; this knob is the global post-pass it always was.

## MotionSection

```tsx
export function MotionSection({ value, onChange, smoothingMs, onSmoothingChange, onApplyToAll }: {
  value: MotionSettings;
  onChange: (v: MotionSettings) => void;
  smoothingMs: number;
  onSmoothingChange: (ms: number) => void;
  onApplyToAll: () => void;
}): JSX.Element
```

### Props

- `value: MotionSettings` - `doc.settings.motion`.
- `onChange: (v: MotionSettings) => void` - called with the full next `MotionSettings` (the same full-object convention the sibling sections use). `EditorSettingsDialog` composes it into `{ ...settings, motion }` and calls `onSaveSettings`.
- `smoothingMs: number` - `doc.settings.zoom.camera_smoothing_ms`. Passed as a scalar rather than the whole `ZoomSettings`, so this section cannot accidentally write a zoom seed.
- `onSmoothingChange: (ms: number) => void` - the dialog's `setSmoothing`, which spreads it back onto `settings.zoom`.
- `onApplyToAll: () => void` - "Apply to all regions". A DOC edit, not a settings write, so it goes through the caller's `applyOp` (`{ op: "apply_motion_default" }`) rather than `onSaveSettings` - which is why it is its own prop threaded from `Editor.tsx` (`onApplyMotion`) through `EditorDialogs`.

### Controls

- **Feel** - a `Picker` of the five presets (`PRESETS`), each option's `title` its `feel` line, with that line also shown as an `.e-lede` under the row. Picking one writes `{ preset: id, ...presetPatch(id) }`, so the name and the two curve strings always land together.
- **The graph** - a read-only `MotionGraph` (`motion/MotionGraph.md`) of the default pair on a 3 s zoom shape (`defaultGraphInput`: in over 450 ms, out over 700 ms, peak 2x), between the preset row and Apply so picking a preset redraws it directly above the button that spreads it.
- **Apply to all regions** - an `.e-modal-btn` calling `onApplyToAll`, with an `.e-lede` saying what it does and that it is one undo step.
- **Camera smoothing** - `Slider` 0-400 step 10, "Off" at 0, otherwise "{v} ms, {lag} ms lag". The `.e-lede` under it names the measured cost in milliseconds once the knob is off zero, and keeps the original "120 ms is a good start" hint while it is off.

### The picker shows the STRINGS, not the stored name

`const current = presetOf(value.easing, value.easing_out)` - not `value.preset`. A curve dragged off a preset in an inspector, or a hand-edited config, must read as Custom rather than as a preset it no longer is; `preset` is provenance, the strings are the truth. When `current` is `"custom"` a sixth "Custom" option is appended so the picker can SHOW it, and only then - it is never something to pick INTO, since there is no such curve to write.

## smoothingLagMs

```ts
export const smoothingLagMs = (ms: number) => number
```

The lag in milliseconds a camera-smoothing window costs: `round(ms * 0.3)`, the slope the jank probe measured (120 ms costs about 33 ms of lag, 250 ms about 83 ms - see `docs/api/src-tauri/src/export/camera/smoothing.md`). Exported so the test can pin it against those measured figures, and so a future inspector readout uses the one definition.

## DEFAULT_MOTION_RESET

```ts
export const DEFAULT_MOTION_RESET: MotionSettings
```

Mirrors Rust `MotionSettings::default()` (`settings/motion.rs`) exactly: `{ preset: "soft", easing: "smooth", easing_out: "smooth" }`. The header's reset icon replaces the WHOLE object with it, so all three fields are here - the same rule `DEFAULT_ZOOM_SETTINGS` follows for `cam_zoom_default` (see `CursorPanel.md` for the bug class).

Soft is the bare word `"smooth"` and not an equivalent `keys(...)` curve; `presets.md` and `settings/motion.md` both carry the reasoning. `MotionSection.test.ts` pins that the reset literal is a pair `presetOf` reads back as Soft, so Reset can never leave the picker showing Custom.

### Used by

- `EditorSettingsDialog` (`src/editor/shell/settings/EditorSettingsDialog.tsx`) - `value={settings.motion}`, `onChange={setMotion}`, `smoothingMs={settings.zoom.camera_smoothing_ms}`, `onApplyToAll={onApplyToAll}`.

## defaultGraphInput

```ts
export const defaultGraphInput = (m: MotionSettings): GraphInput
```

The default pair as the read-only graph draws it: a 3 s zoom to 2x, in over 450 ms, out over 700 ms - the shape a zoom made from these defaults would have, so the picture matches the promise the row makes.
