# src/editor/shell/ShortcutsOverlay.tsx

Small centered modal listing the editor's keyboard shortcuts - opened via `?` (`useEditorKeymap`'s `"overlay"` action, see `keymap.ts`), addressing "shortcuts are undiscoverable". Reuses the shared `.e-modal`/`.e-modal-scrim` pattern (`ConfirmDialog`/`ExportDialog`) with the same 0.16s tween, so it reads as the same modal family rather than a bespoke popup.

## ShortcutsOverlay

```tsx
export function ShortcutsOverlay({ open, onClose }: { open: boolean; onClose: () => void }): JSX.Element
```

### Props

- `open: boolean` - whether the overlay is shown; `Editor.tsx` owns this as `showShortcuts`, toggled by the `?` shortcut.
- `onClose: () => void` - called on a scrim click or Escape.

### Behavior

- Renders a fixed `GROUPS` list as `.e-shortcuts-list` rows, each a label + a `<kbd>.e-shortcuts-key` chip, under a `.e-shortcuts-group` label reusing the editor's existing micro-label recipe. Two groups, because M1a gave the editor two kinds of key:
  - **Editing** - `Space` Play/Pause, `Z` Add zoom, `S` Add spotlight, `Shift+drag` Choose a range on the ruler, `Del` Remove selected, `Esc` Deselect, `Ctrl+Z` Undo, `Ctrl+Shift+Z` Redo, `?` Shortcuts.
  - **Layout** - `Ctrl+Space` Maximize area, `Ctrl+1..9` Switch workspace, `Tab` Focus Properties.
- Every KEY here is a row of the one keymap table (`hooks/keymap.ts`), so this list cannot drift into describing a binding that does not exist - if a binding moves, the table is what moved. Esc arrived with M1a (selection became its own axis, so leaving it needed its own key); the whole Layout group is the shell's, A3 and A5.
- The one exception is `Shift+drag` (the time remap, `timeline/useRangeSelect.ts`): a pointer GESTURE, so it has no table row. It is listed anyway because it is the only way to aim the transport's Cut and Speed precisely, and it has no affordance on screen beyond the ruler's own `title`.
- An effect attaches a `window` `keydown` listener ONLY while `open` (so it never listens when closed) and calls `onClose` on `Escape`.
- The scrim (`.e-modal-scrim`) closes on `onPointerDown`; the card itself (`.e-modal.e-shortcuts-modal`) stops that pointerdown from bubbling, same as `ConfirmDialog`/`ExportDialog`.
- `AnimatePresence` + the same fade (scrim, 0.14s) / scale+y+fade (card, 0.16s tween, `ease: [0.4, 0, 0.2, 1]`) as every other editor modal.
- A row is `min-height: 32px`, not a fixed 32 (width audit, 2026-09-14): a description that needs two lines in a 300px card - "Choose a range on the ruler" is already most of one - used to grow out of its row and overlap its neighbours. The `kbd` never shrinks or wraps; the description is what gives.

### Used by

`Editor` (`src/editor/Editor.tsx`) - mounted once alongside `ExportDialog`/`moveOffDialog`, wired to `showShortcuts`/`setShowShortcuts` and `useEditorKeymap`'s `onOverlay`.

The Layout group (Ctrl+Space, Ctrl+1..9, Tab) went with the area shell; one Editing group remains, Shift+drag included.
