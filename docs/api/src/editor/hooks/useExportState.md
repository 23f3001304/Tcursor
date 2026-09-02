# src/editor/hooks/useExportState.ts

Export-run state (progress/done/error/path) plus the `export-*` IPC listeners.

## useExportState

```ts
export function useExportState(onExportWarning?: (msg: string) => void)
```

Split out of `useEditorData.ts` (Task 11 - that file was already at its own line budget) so this concern has its own home. Owns `exporting`/`pct`/`exportDone`/`exportError`/`exportPath`, subscribes to `export-progress`/`export-done`/`export-error`/`export-warning` once (empty-deps effect; `onExportWarning` is read through a ref so a fresh callback identity each render doesn't tear down and re-subscribe), and returns `startExport`/`exportStartedAt`.

### Why `startExport`/`exportStartedAt` exist (D Low)

`ExportProgress`'s ETA estimate needs an elapsed-time baseline. It used to track "when did this export start" in its OWN `useRef`, set on the first render where `exporting` was true. But `ExportDialog` is conditionally rendered (`{open && <Modal>...}`) - closing it while an export keeps running in the background is a supported, documented flow (see `ExportDialog.md`), and closing it unmounts `ExportProgress`, destroying that ref. Reopening remounted it fresh, so the ETA baseline silently reset to "just started" even hours into a long export. `exportStartRef` lives here instead, in a hook `Editor.tsx` calls directly (not nested inside the dialog's own tree), so it survives the dialog's mount/unmount cycles. `startExport()` (called from `Editor.tsx`'s `onExport`, replacing the old inline `setExporting(true); setPct(0)`) sets `exporting`, resets `pct`, and stamps `exportStartRef.current = Date.now()` all at once.

### `onExportWarning`

Rust's `export-warning` event (webcam missing/frozen in an otherwise-successful export - Task 9 added the `emit`, nothing on this side ever listened) fires with a plain string payload, BEFORE `export-done`, and never touches `exporting`/`exportDone` (the export still succeeded - this is purely "here's what's off about the file you got"). `Editor.tsx` passes the Toast pill's `push` (`useUndoToast`), so the user sees it as a transient pill instead of it only ever reaching stderr.

### Returns

`{ exporting, setExporting, pct, setPct, exportDone, setExportDone, exportError, setExportError, exportPath, setExportPath, startExport, exportStartedAt }`
