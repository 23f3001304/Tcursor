# src/editor/panels/LayoutPresetList.tsx

The saved-looks half of the Layouts panel: the built-in row, one row per saved look, and the two write actions under them. Split out of `LayoutsPanel.tsx` so the panel file holds the flow and this one holds the list's own interaction state.

## NameField

```tsx
function NameField({ initial, check, onCommit, onCancel }: {
  initial: string;
  check: (name: string) => string | null;
  onCommit: (name: string) => void;
  onCancel: () => void;
}): JSX.Element
```

The section's one text field, used for **both** "save the current look" (`initial: ""`) and "rename this look" (`initial: p.name`). Enter commits, Esc cancels, and a refused name says why underneath (`.e-lay-err`, `role="alert"`) instead of silently doing nothing.

The rule comes in as `check`, not as a hardcoded call: the caller passes `presetNameError(n, presets)` for a new look and `presetNameError(n, presets, p.id)` for a rename, so both fields enforce the same rule and only the "except myself" clause differs. The error clears on the next keystroke. It autofocuses because it only ever exists in the first place because the user just asked for it, and `maxLength` is `MAX_PRESET_NAME + 1` deliberately - one character past the limit, so a user who overshoots gets the sentence explaining the limit rather than a field that silently stops accepting letters.

Not exported: it is this file's private idiom, and a second caller would want the panel's own field styling anyway.

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

### Why Apply is a button and not a selected state

Nothing in the list is ever "current". A look is a snapshot, and the moment any knob moves the project has drifted from whatever was applied - so a selected row would be a claim the panel cannot keep true. Every row therefore offers the same one verb.

### Interaction state

Three pieces of local state, and **exactly one of them is open at a time** (`close()` clears all three before opening anything): `adding` (the new-look field), `renaming` (which row is showing a rename field in place of its name), and `menu` (which row's actions are showing). A 320px panel cannot afford two open things fighting for the same width.

A row's actions (Rename, Delete) drop **under** the row rather than into a popover: there is nowhere in a 320px column to float a menu that would not cover the list it belongs to.

`madeDefault` is the fourth, and is only a hint line - "Make this the default for new recordings" writes the app config's `appearance`, which is invisible until the next recording starts, so the confirmation has to be in words. The sentence names the undo (apply Default, press it again) rather than offering a dialog, because the action is already reversible.

### Motion

One `AnimatePresence` over the rows (height + opacity, plus `layout` so the survivors slide rather than jump when one is deleted), one per row over its action strip, and one over the save button / name field pair - all on the panels' 0.16s content tween, zeroed under `useReducedMotion`. The save pair is deliberately NOT `mode="wait"`: the field must exist the instant the button is pressed, since the button is what the user is looking at and the field autofocuses, so the two cross-fade instead of the field queueing behind the button's exit.

### Used by

- `src/editor/panels/LayoutsPanel.tsx`.
