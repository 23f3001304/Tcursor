# src/editor/shell/dialogs/ExportProgress.tsx

The export progress/outcome view rendered inside `ExportDialog` once an export has started, finished, or failed. `startedAt` (the export's own start time) is a PROP now (Task 11, D Low), owned by the caller's `useExportState` - not tracked in an internal ref here - so `exportEta`'s `estimateEtaMs` has a baseline that survives this component unmounting/remounting.

## ExportProgress

```tsx
export function ExportProgress({ exporting, pct, done, error, exportPath, startedAt, onReset, onClose }: {
  exporting: boolean; pct: number; done: boolean; error: string | null; exportPath: string;
  startedAt: number | null;
  onReset: () => void; onClose: () => void;
}): JSX.Element
```

Renders one of three views depending on props: a live progress wave, a success outcome, or an error outcome.

### Props

- `exporting: boolean` - whether the export is currently running. *Why:* drives which of the three views renders, and starts/stops the internal elapsed-time clock.
- `pct: number` - progress percentage 0-100 (from the `export-progress` Tauri event, via `useEditorData`). *Why:* shown directly and fed to `estimateEtaMs` for the ETA line.
- `done: boolean` - whether the export finished successfully. *Why:* switches to the outcome view once `exporting` has gone false without an error. The separate success icon is gone: the wave is already flat at 100% and fades out, so the dot it left behind is what scales away as the checkmark draws itself in its place.
- `error: string | null` - the export error message, or `null`. *Why a string not a boolean:* the message is shown directly in the error view so the user knows what failed, not just that something did.
- `exportPath: string` - the finished export's own absolute file path (`export-done`'s payload, `<folder>/final.<ext>` - see `run.rs`), or `""` before any export has completed. Gates whether the done view's "Show in folder" button renders at all.
- `startedAt: number | null` (Task 11, D Low) - wall-clock `Date.now()` of when the CURRENT export began, from `Editor.tsx`'s `useExportState().exportStartedAt` (stamped once, in `startExport`, at the same moment `exporting` flips true). `null` before any export has started. Threaded through `ExportDialog` unchanged. *Why a prop, not local state:* see the file header - a ref here used to reset on remount, silently zeroing the ETA baseline whenever the dialog was closed and reopened mid-export.
- `onReset: () => void` - called from the "Try again" (error) / "Export again" (done) button. *Why:* owned by the caller (`ExportDialog`/`Editor`) since it clears the lifted `exportDone`/`exportError`/`exportPath` state, returning `ExportDialog` to its settings-form view.
- `onClose: () => void` - called from "Close"/"Done". *Why:* dismisses the whole dialog; owned by the caller since the open/closed state lives in `Editor`.

### Behavior

**Progress view** (`!done && !error`): an "Exporting..." + `{pct}%` row, a `SweepWave` at `pct`, and an ETA line from `estimateEtaMs(Date.now() - startedAt, pct)` - shows "Estimating time remaining..." until `pct > 0` or `startedAt` is `null`.

The wave replaced the fill bar (`.e-export-bar-track` / `.e-export-bar-fill`, both deleted from `editor.css`) and the `Spin` beside the label. It is not decoration standing in for the bar: the dot sits exactly at `pct` of the width on the wave's trailing crest, and the whole wave's amplitude decays to flat as `pct` reaches 100, so how far along the export is reads from the shape before the number is read - and there is one indicator rather than two that could disagree. Panel-design benchmark (c) item 7: export progress is a generic bar industry-wide. `TopBar`'s own mini fill in the export BUTTON is untouched; it is a different affordance (a button underline), not this dialog's progress readout.

**Done view** (`done`): the same `SweepWave` at `pct={100} done`, "Export complete", and either two or three buttons - "Export again" (`onReset`), "Show in folder" (calls `revealItemInDir(exportPath)` from `@tauri-apps/plugin-opener`, swallowing a rejection - only rendered when `exportPath` is non-empty), and "Done" (`onClose`, primary).

**Error view** (`error`, checked first so an error always wins over a stale `done`): a warning icon, "Export failed", the raw `error` message, and two buttons - "Close" (`onClose`) and "Try again" (`onReset`, primary).

Both primary buttons ("Done", "Try again") are `motion.button`s carrying the app-wide press spring (design/premium-pass D6, `whileTap: { scale: 0.96 }`); the secondary buttons ("Close", "Export again", "Show in folder") stay unmotioned.

### Implementation

One effect recomputes `etaMs` via `estimateEtaMs` whenever `exporting`/`pct`/`startedAt` change, using `null` whenever not actively exporting or `startedAt` is still `null`. The start timestamp itself is entirely the caller's concern now (`useExportState.md`).

### Used by

- `ExportDialog` (`src/editor/shell/dialogs/ExportDialog.tsx`) - rendered in place of the settings form whenever `exporting || done || error`.
