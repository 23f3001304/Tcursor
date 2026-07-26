# src/editor/shell/ExportProgress.tsx

The export progress/outcome view rendered inside `ExportDialog` once an export has started, finished, or failed. Owns the "when did this export start" clock (an internal ref, reset whenever `exporting` flips false -> true) so `exportEta`'s `estimateEtaMs` has an elapsed baseline without the caller needing to track wall-clock time itself.

## ExportProgress

```tsx
export function ExportProgress({ exporting, pct, done, error, onReset, onClose }: {
  exporting: boolean; pct: number; done: boolean; error: string | null;
  onReset: () => void; onClose: () => void;
}): JSX.Element
```

Renders one of three views depending on props: a live progress bar, a success outcome, or an error outcome.

### Props

- `exporting: boolean` - whether the export is currently running. *Why:* drives which of the three views renders, and starts/stops the internal elapsed-time clock.
- `pct: number` - progress percentage 0-100 (from the `export-progress` Tauri event, via `useEditorData`). *Why:* shown directly and fed to `estimateEtaMs` for the ETA line.
- `done: boolean` - whether the export finished successfully. *Why:* switches to the checkmark outcome view once `exporting` has gone false without an error.
- `error: string | null` - the export error message, or `null`. *Why a string not a boolean:* the message is shown directly in the error view so the user knows what failed, not just that something did.
- `onReset: () => void` - called from the "Try again" (error) / "Export again" (done) button. *Why:* owned by the caller (`ExportDialog`/`Editor`) since it clears the lifted `exportDone`/`exportError` state, returning `ExportDialog` to its settings-form view.
- `onClose: () => void` - called from "Close"/"Done". *Why:* dismisses the whole dialog; owned by the caller since the open/closed state lives in `Editor`.

### Behavior

**Progress view** (`!done && !error`): a `Spin` + "Exporting..." + `{pct}%` row, a Motion-animated fill bar (`.e-export-bar-fill`, spring transition matching `TopBar`'s own mini bar), and an ETA line from `estimateEtaMs(Date.now() - startRef.current, pct)` - shows "Estimating time remaining..." until `pct > 0`.

**Done view** (`done`): a check icon, "Export complete", and two buttons - "Export again" (`onReset`) and "Done" (`onClose`, primary).

**Error view** (`error`, checked first so an error always wins over a stale `done`): a warning icon, "Export failed", the raw `error` message, and two buttons - "Close" (`onClose`) and "Try again" (`onReset`, primary).

### Implementation

Two effects manage the elapsed-time clock: one sets `startRef.current = Date.now()` the moment `exporting` becomes true (and clears it back to `null` when it becomes false); the other recomputes `etaMs` via `estimateEtaMs` whenever `exporting`/`pct` change, using `null` whenever not actively exporting.

### Used by

- `ExportDialog` (`src/editor/shell/ExportDialog.tsx`) - rendered in place of the settings form whenever `exporting || done || error`.
