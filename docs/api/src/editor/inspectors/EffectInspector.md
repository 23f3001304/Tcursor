# src/editor/inspectors/EffectInspector.tsx

Left-panel inspector for the selected effect region (spotlight), shown in place of the tab content
while an effect region is selected. Edits start/end/fade/mode, three overridable per-region
spotlight params (dim/radius/feather, each falling back to the global default when unset), the
webcam-dim toggle (a global setting, not per-region), and delete.

## OverrideField

```tsx
function OverrideField({ label, value, defaultValue, min, max, step, onToggle, onChange }: {
  label: string; value: number | undefined; defaultValue: number;
  min: number; max: number; step: number;
  onToggle: (on: boolean) => void; onChange: (v: number) => void;
}): JSX.Element
```

Internal to this file, not exported. One overridable spotlight param (used for `dim`/`radius`/`feather`, each as a `0..1` fraction).
`value === undefined` means "inherit the global default" (`defaultValue`) - the `Switch` on the
top row toggles between that and an explicit per-region override. The `Slider` below is always
bound to `value ?? defaultValue`, `disabled` while `value === undefined` (Slider.tsx's own
`disabled` styling - half opacity, a dimmed `--e-dim` thumb border instead of the accent color,
`cursor: default`, and pointer/keyboard interaction blocked outright - was already wired here
before Task 11; verified still correct, ux audit #22's "enabled-looking slider" read was really
the duplicate-label issue below muddying the whole control's legibility, not a missing disabled
state).

**One name, once (Task 11, ux audit #22's "leftover" note).** The switchrow used to show `label`
itself (e.g. "Dim") right above `Slider`'s own `label` row, which ALSO prints `label` (e.g. "Dim
65%") - the field name rendered twice. The switchrow now reads the generic "Override" instead,
with the real field name moved onto the `Switch` itself as `ariaLabel={`Override ${label}`}` (a11y
- `Switch` has no other way to associate a name, since wrapping it in a `<label>` doesn't
auto-associate a custom `role="switch"` button the way it would a native input) - `label` now
prints exactly once, in `Slider`'s own row.

**Live readout (render hygiene pass, fix round 2).** The value/"Default (X%)" text used to be a
static `<b>` next to the `Switch`, reading `value`/`defaultValue` directly - it never moved during
a drag, since `onChange` here is `applyOp`-backed (a full IPC round trip, itself now debounced ~80ms
- see `Slider.md`). It's now `Slider`'s own `label`/`formatValue` readout instead, sourced from the
LIVE (optimistic) value, so it tracks the thumb in real time: `formatValue` renders `"X%"` while
overridden, or `"Default (X%)"` while not (matching the old text almost exactly - the row layout
changed from "one row: label, value, switch" to "two rows: label + switch, then label + live
value" since `Slider`'s `label` always renders immediately above its own track, not inline with an
external switch).

**Wrapper is a `<div>`, not a `<label>` (fix round 4).** The outer element used to be
`<label className="e-field">` wrapping both the `Switch` and the `Slider`. HTML forwards a click
anywhere inside a `<label>` to its first labelable control, so releasing a `Slider` drag (the
mouseup/click on the track) also fired the `Switch`'s `onClick`, toggling the override off and
wiping the just-dragged value via the `-1` sentinel (and the reverse: clicking the disabled slider
area toggled the override on). Keyboard interaction with the slider didn't trigger this, since it
doesn't synthesize a click. Swapped to `<div className="e-field">` - `.e-field`'s CSS is a plain
class selector (not `label.e-field`), and both controls already carry their own `aria-label`, so no
label semantics are lost.

## EffectInspector

```tsx
export function EffectInspector({ effect, dur, settings, onApply, onDimCamera, onClose }: {
  effect: EffectRegion; dur: number; settings: Settings; onApply: (op: EditOp) => Promise<EditDoc | null>;
  onDimCamera: (v: boolean) => void; onClose: () => void;
}): JSX.Element
```

### Props

- `effect: EffectRegion` - the selected region; `dim`/`radius`/`feather` are `Option<f32>` on the
  Rust side (`undefined` here when unset - see `EditorPanels.md`'s note on the `-1` sentinel used
  to clear one back to `undefined` via `upd`).
- `dur: number` - clip duration (bounds the end input).
- `settings: Settings` - supplies each `OverrideField`'s `defaultValue`
  (`settings.clickfx.spotlight_dim`/`spotlight_radius`/`spotlight_feather`) and the "Dim webcam"
  switch's current state (`settings.clickfx.spotlight_dim_camera`).
- `onApply` - every field (start/end/fade/mode/the three overrides/delete) applies an
  `update_effect`/`remove_effect` op through this.
- `onDimCamera: (v: boolean) => void` - the "Dim webcam" switch's own writer - a GLOBAL setting
  (`saveDocSettings` in `EditorPanels`, not `onApply`), since it applies to every spotlight, not
  just this region.
- `onClose` - deselect (clears the selection in `Editor`).

### Behavior

Start/End and Fade in/Fade out are two `NumberField` pairs (`e-field2` rows). Spotlight Mode is a
`Picker` over the global mode list plus `"Use Global Default"`. Dim/Radius/Feather are three
`OverrideField`s (see above) - `onToggle(on)` for each writes `defaultValue` when turning an
override ON (so the slider starts from a sane value instead of jumping to `0`) or the `-1`
sentinel when turning it OFF (converted back to `None` server-side). "Dim webcam" is a plain
`Switch` + explanatory `e-lede` text, unrelated to the three overrides. Delete removes the region
and closes the inspector.
