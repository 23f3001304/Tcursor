# src/editor/hooks/input/useEditorModals.ts

Which of the editor's three modals is open, and the one boolean the rest of the editor cares about. Split out of `Editor.tsx`: three `useState`s, six inline close handlers and one derived flag are a small state machine, and the component only ever uses its answers.

## useEditorModals

```ts
export function useEditorModals(blocked: boolean): {
  dialog: { showExport, onCloseExport, showShortcuts, onCloseShortcuts, showSettings,
            onCloseSettings, onOpenShortcuts },
  shortcutsOpen: boolean; modalOpen: boolean;
  openExport: () => void; openSettings: () => void; toggleShortcuts: () => void;
}
```

`blocked` is `useMoveModeGuard`'s own `pending` state (`moveOffOpen`): a fourth thing that makes the editor modal without being one of the three dialogs this hook owns. `dialog` is spread straight onto `EditorDialogs`, which is why its field names are that component's prop names rather than this hook's own.

### Notes

- `showExport` is the only export-dialog state the editor owns itself; `exporting`/`pct`/`exportDone`/`exportError` are all lifted from `useExportState` (Task 11 - previously `useEditorData`) and reach `ExportDialog` through `EditorDialogs`.
- `showShortcuts` is the analogous local state for `ShortcutsOverlay` - toggled by `useEditorKeymap`'s `onOverlay` callback (fires when `resolveKeyAction` returns `"overlay"`, i.e. the `?` key) and cleared by the overlay's own `onClose` (scrim click or Escape).
- `showSettings` (Task 35) is the analogous local state for `EditorSettingsDialog` - toggled by `TopBar`'s gear button (`onOpenSettings`) and cleared by the dialog's own `onClose` (scrim click or Escape), same shape as `showExportDialog`/`showShortcuts`.

- **`modalOpen` (bug-sweep-2 Task 8, M4; extended in review round 1, Important C; narrowed in M4 T5).** `showExportDialog || showShortcuts || showSettings || moveOffOpen` (`moveOffOpen` is `useMoveModeGuard`'s own `pending` state, re-exposed alongside `moveOffDialog` so `Editor` can fold it into this check), passed to `useEditorKeymap` alongside a separate `shortcutsOpen: showShortcuts`. Every global shortcut (Space/Z/S/Delete/`?`) is inert while ANY of these four is open - previously only typing in a text field was checked, so e.g. `z` typed while the Export dialog was open silently added a zoom behind it. The AI Director is deliberately NOT in this list any more: its review sheet is a panel, not a modal, and the keymap must stay live while the user reads it. Its replay scrim catches pointer input and Esc on its own (`DirectorScrim`); a shortcut that lands during the short replay applies harmlessly to a doc the replay never touches. `shortcutsOpen` is passed SEPARATELY (not just folded into `modalOpen`) so `?` can still toggle `ShortcutsOverlay` CLOSED while it, specifically, is the open modal - see `useEditorKeymap.md`/`keymap.md`'s `resolveKeyAction`. Since M1a A3 the same boolean also goes into the `shell` bundle (`ShellProps.modalOpen`): the editor shell owns one key of its own, `Ctrl+Space` to maximize an area, and it has to go inert behind a dialog for exactly the same reason every other shortcut does.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller.
