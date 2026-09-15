# src/editor/shell/dialogs/ExportDialog.tsx

The detailed export settings dialog, opened from `TopBar`'s Export button (`onOpenExport`). Collects `resolution`/`fps`/`quality_crf`/`format` into a local `ExportSettings` draft, then hands it to `onExport` on confirm. Export state itself (`exporting`/`pct`/`done`/`error`) is NOT owned here - it is lifted in `useEditorData`/`Editor` - so this component is a view over that state plus its own settings-form draft, swapping its body between the form and `ExportProgress` once an export is running, has finished, or has failed.

## ExportDialog

```tsx
export function ExportDialog({ open, exporting, pct, done, error, exportPath, startedAt, onClose, onExport, onReset }: {
  open: boolean; exporting: boolean; pct: number; done: boolean; error: string | null; exportPath: string;
  startedAt: number | null;
  onClose: () => void; onExport: (settings: ExportSettings) => void; onReset: () => void;
}): JSX.Element
```

### Props

- `open: boolean` - whether the dialog is visible. *Why controlled, not self-managed:* `TopBar`'s Export button and the dialog's own close/scrim both need to affect the same boolean, owned by `Editor`.
- `exporting: boolean`, `pct: number`, `done: boolean`, `error: string | null`, `exportPath: string`, `startedAt: number | null` (Task 11) - passed straight through to `ExportProgress` (`startedAt` is `useExportState`'s `exportStartedAt` - see `ExportProgress.md`/`useExportState.md` for why it's owned there, not here or inside `ExportProgress`); `exporting`/`done`/`error` are also used here to decide whether to show the settings form or the progress/outcome view (`showProgress = exporting || done || !!error`), and `done`/`error` gate `handleClose`'s reset (see Implementation).
- `onClose: () => void` - the RAW dismiss callback the caller (`Editor`) hands down; this component never calls it directly - every dismissal path goes through the internal `handleClose` wrapper instead (see Implementation). *Why closing does not cancel an in-flight export:* there is no cancel mechanism on the backend; closing early just hides the dialog while `TopBar`'s own mini progress bar keeps reflecting the running export.
- `onExport: (settings: ExportSettings) => void` - called when the settings form's Export button is clicked, with the current draft. *Why the caller owns the actual `exportProject` call:* it also needs to reset the lifted `exportDone`/`exportError`/`exportPath` state and flip `exporting`/`pct`, which this component does not have setters for.
- `onReset: () => void` - clears a `done`/`error`/`exportPath` outcome, returning to the settings form on the NEXT render (this component does not close itself; the caller clears the state that `showProgress` depends on). Called from `ExportProgress`'s "Try again"/"Export again" buttons directly (does NOT close the dialog - the user stays in it to re-export), and from `handleClose` (see Implementation) when the dialog is dismissed while showing an outcome.

### Implementation

Reuses the app-wide `.e-modal-scrim`/`.e-modal` pattern (`ConfirmDialog`'s styling) with a wider `.e-export-modal` variant. The settings form's primary Export button is a `motion.button` carrying the app-wide press spring (design/premium-pass D6, `whileTap: { scale: 0.96 }`); Cancel stays unmotioned. Four controls: a `Picker` each for resolution (`source`/`p720`/`p1080`/`p1440`/`p2160`), frame rate (`source`/`f30`/`f60`), and format (`mp4`/`webm`/`gif`); a `Slider` (18-28, step 1) for `quality_crf`. The quality slider is `disabled` and annotated "Not used for GIF" when `format === "gif"` (GIF quality comes from the palette filter, not a CRF knob - see `encode::ffmpeg_args::export_args`). The draft starts at `DEFAULT_EXPORT_SETTINGS` (mirrors the Rust `ExportSettings::default()` - Source/60fps/CRF 24/MP4, i.e. today's export unchanged) every time the component mounts.

**Reset-on-close-of-an-outcome (`handleClose`).** `const handleClose = () => { if (done || error) onReset(); onClose(); };`, used for the scrim's `onPointerDown`, the header X button, the settings form's Cancel button, and passed to `ExportProgress` as ITS `onClose` (so its Close/Done buttons go through the same function). A finished/failed export's outcome must survive the dialog being closed and reopened: closing while still `exporting` is a supported "check back later" flow, and the export can finish in the BACKGROUND while the dialog is closed entirely - reopening it must still show that outcome (e.g. so "Show in folder" is actually reachable), which an earlier reset-ON-OPEN design would have wiped before the user ever saw it. `handleClose` instead resets ONLY when the dialog is being dismissed WHILE it is currently displaying `done` or `error` - never while merely `exporting` (that leaves the outcome-in-waiting untouched for a later reopen) and never for the plain settings-form Cancel (nothing to reset). Concretely: close mid-export -> state persists; export finishes in the background -> reopen shows the outcome; closing THAT outcome view -> resets; the next open is a fresh form.

### Used by

- `Editor` (`src/editor/Editor.tsx`) - rendered once near `moveOffDialog`; `TopBar`'s `onOpenExport` sets the `open` state, and `onExport` wires to `exportProject(folder, settings)` after resetting `exportDone`/`exportError`/`exportPath` and calling `useExportState`'s `startExport()` (Task 11 - replaces the old inline `setExporting(true); setPct(0)`, additionally stamping the ETA baseline). `onReset` likewise clears `exportDone`/`exportError`/`exportPath`.
