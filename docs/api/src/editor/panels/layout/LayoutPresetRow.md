# src/editor/panels/layout/LayoutPresetRow.tsx

One row of the saved-looks list: the look's name, its Apply button, its kebab, and the action strip that drops out under it. Split out of `LayoutPresetList.tsx` so that file holds the list's interaction state and this one holds what a single row looks like in each of its three states (named, renaming, menu open).

## LayoutPresetRow

```tsx
export function LayoutPresetRow({ preset, still, timing, renaming, menuOpen, checkName, onApply, onRename, onDelete, onOpenMenu, onStartRename, onClose }: {
  preset: LayoutPreset;
  still: boolean;
  timing: Transition;
  renaming: boolean;
  menuOpen: boolean;
  checkName: (name: string) => string | null;
  onApply: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
  onOpenMenu: () => void;
  onStartRename: () => void;
  onClose: () => void;
}): JSX.Element
```

### Props

- `preset: LayoutPreset` - the row's look. `BUILTIN_PRESET.id` is the one value that suppresses the kebab: Default cannot be renamed or deleted, and it is undeletable because it is never in the saved array at all, not because of a guard here.
- `still` / `timing` - the list's resolved motion settings, passed down rather than each row calling `useReducedMotion` itself. One source of truth for the 0.16s content tween, and rows animate in step.
- `renaming` / `menuOpen` - which of the row's two open states is showing. Both are the LIST's state, because only one row in the section may be open at a time.
- `checkName` - the name rule for THIS row (`presetNameError(n, presets, preset.id)`, i.e. "not a duplicate of anything except myself"), handed to `PresetNameField`.
- `onApply` / `onRename` / `onDelete` - the row's three writes. Each already closes the section on the way out; this file never calls `close()` itself.
- `onOpenMenu` / `onStartRename` / `onClose` - the three state changes, likewise the list's.

### Why Apply is a button and not a selected state

Nothing in the list is ever "current". A look is a snapshot, and the moment any knob moves the project has drifted from whatever was applied - so a selected row would be a claim the panel cannot keep true. Every row therefore offers the same one verb.

### The action strip

Rename and Delete drop **under** the row rather than into a popover: there is nowhere in a 320px column to float a menu that would not cover the list it belongs to. Its own `AnimatePresence` animates height and opacity on the list's `timing`.

### Used by

- `src/editor/panels/layout/LayoutPresetList.tsx` - one per row, including the built-in Default.
