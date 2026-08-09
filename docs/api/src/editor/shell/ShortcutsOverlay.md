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

- Renders a fixed `SHORTCUTS` list (`Space` Play/Pause, `Z` Add zoom, `S` Add spotlight, `Del` Remove selected, `Ctrl+Z` Undo, `Ctrl+Shift+Z` Redo, `?` Shortcuts) as `.e-shortcuts-list` rows, each a label + a `<kbd>.e-shortcuts-key` chip.
- An effect attaches a `window` `keydown` listener ONLY while `open` (so it never listens when closed) and calls `onClose` on `Escape`.
- The scrim (`.e-modal-scrim`) closes on `onPointerDown`; the card itself (`.e-modal.e-shortcuts-modal`) stops that pointerdown from bubbling, same as `ConfirmDialog`/`ExportDialog`.
- `AnimatePresence` + the same fade (scrim, 0.14s) / scale+y+fade (card, 0.16s tween, `ease: [0.4, 0, 0.2, 1]`) as every other editor modal.

### Used by

`Editor` (`src/editor/Editor.tsx`) - mounted once alongside `ExportDialog`/`moveOffDialog`, wired to `showShortcuts`/`setShowShortcuts` and `useEditorKeymap`'s `onOverlay`.
