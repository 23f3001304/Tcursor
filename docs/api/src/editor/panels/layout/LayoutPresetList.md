# src/editor/panels/layout/LayoutPresetList.tsx

The saved-looks half of the Layouts panel: the built-in row, one row per saved look, and the two write actions under them. Split out of `LayoutsPanel.tsx` so the panel file holds the flow and this one holds the list's own interaction state. The row itself is `LayoutPresetRow.tsx` and the text field `PresetNameField.tsx`; what stays here is the list, the add form, and the one rule about what may be open.

## LayoutPresetList

```tsx
export function LayoutPresetList({ presets, appearance, onApply, onSave, onRename, onDelete, onMakeDefault }: {
  presets: LayoutPreset[];
  appearance: AppearanceSettings;
  onApply: (p: LayoutPreset) => void;
  onSave: (name: string) => void;
  onRename: (id: string, name: string) => void;
  onDelete: (id: string) => void;
  onMakeDefault: (a: AppearanceSettings) => void;
}): JSX.Element
```

### Props

- `presets: LayoutPreset[]` - the SAVED looks (`Settings.layout_presets`). `BUILTIN_PRESET` is prepended here for display; it is never in this array, which is what makes it undeletable without a special case.
- `appearance: AppearanceSettings` - the project's current look, i.e. what "Save current look" snapshots and what "Make this the default" writes. Passed rather than read, so this component never touches the doc.
- `onApply` / `onSave` / `onRename` / `onDelete` / `onMakeDefault` - the five writes, all built in `LayoutsPanel` from the pure helpers in `layoutPresets.ts`.

### Interaction state

Three pieces of local state, and **exactly one of them is open at a time** (`close()` clears all three before opening anything): `adding` (the new-look field), `renaming` (which row is showing a rename field in place of its name), and `menu` (which row's actions are showing). A 320px panel cannot afford two open things fighting for the same width. The rows are stateless about this - each is told whether it is the renaming one and whether its menu is open.

`madeDefault` is the fourth, and is only a hint line - "Make this the default for new recordings" writes the app config's `appearance`, which is invisible until the next recording starts, so the confirmation has to be in words. The sentence names the undo (apply Default, press it again) rather than offering a dialog, because the action is already reversible.

### Motion

One `AnimatePresence` over the rows (height + opacity, plus `layout` so the survivors slide rather than jump when one is deleted - both inside `LayoutPresetRow`), one per row over its action strip, and one over the save button / name field pair - all on the panels' 0.16s content tween, zeroed under `useReducedMotion`. `still` and `timing` are resolved once here and handed down, so every row animates in step. The save pair is deliberately NOT `mode="wait"`: the field must exist the instant the button is pressed, since the button is what the user is looking at and the field autofocuses, so the two cross-fade instead of the field queueing behind the button's exit.

### Used by

- `src/editor/panels/layout/LayoutsPanel.tsx`.
