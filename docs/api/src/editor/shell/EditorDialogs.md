# src/editor/shell/EditorDialogs.tsx

The editor's modal stack - Export, Shortcuts and Settings - lifted out of `Editor.tsx` as one unit (T34 L3) so that file stays under its 200-line budget. Behaviour is unchanged by the move.

## EditorDialogs

```tsx
export function EditorDialogs({ folder, settings, exportState, showExport, onCloseExport,
  showShortcuts, onCloseShortcuts, showSettings, onCloseSettings, onOpenShortcuts, onSaveSettings }: { ... }): JSX.Element
```

Owns no state of its own. Every open flag and setter still lives in `Editor`, which is also what `modalOpen` - the keymap's inert-behind-a-modal gate (`keymap.md`) - is computed from, so lifting the JSX out could not change what the keyboard does behind a dialog.

`exportState` is the whole `useExportState` return, passed as one object rather than eleven props. The export kickoff lives here because it is exactly the three result-state resets (`setExportDone(false)` / `setExportError(null)` / `setExportPath("")`) plus the `exportProject` IPC call, shared verbatim by the dialog's `onReset` and its `onExport` - and nothing outside this file reads those three.
