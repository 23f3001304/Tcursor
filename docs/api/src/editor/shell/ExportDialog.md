# src/editor/shell/ExportDialog.tsx

The detailed export settings dialog, opened from `TopBar`'s Export button (`onOpenExport`). Collects `resolution`/`fps`/`quality_crf`/`format` into a local `ExportSettings` draft, then hands it to `onExport` on confirm. Export state itself (`exporting`/`pct`/`done`/`error`) is NOT owned here - it is lifted in `useEditorData`/`Editor` - so this component is a view over that state plus its own settings-form draft, swapping its body between the form and `ExportProgress` once an export is running, has finished, or has failed.

## ExportDialog

```tsx
export function ExportDialog({ open, exporting, pct, done, error, onClose, onExport, onReset }: {
  open: boolean; exporting: boolean; pct: number; done: boolean; error: string | null;
  onClose: () => void; onExport: (settings: ExportSettings) => void; onReset: () => void;
}): JSX.Element
```

### Props

- `open: boolean` - whether the dialog is visible. *Why controlled, not self-managed:* `TopBar`'s Export button and the dialog's own close/scrim both need to affect the same boolean, owned by `Editor`.
- `exporting: boolean`, `pct: number`, `done: boolean`, `error: string | null` - passed straight through to `ExportProgress`; also used here to decide whether to show the settings form or the progress/outcome view (`showProgress = exporting || done || !!error`).
- `onClose: () => void` - dismisses the dialog (scrim click, X button, or an `ExportProgress` Close/Done button). *Why closing does not cancel an in-flight export:* there is no cancel mechanism on the backend; closing early just hides the dialog while `TopBar`'s own mini progress bar keeps reflecting the running export.
- `onExport: (settings: ExportSettings) => void` - called when the settings form's Export button is clicked, with the current draft. *Why the caller owns the actual `exportProject` call:* it also needs to reset the lifted `exportDone`/`exportError` state and flip `exporting`/`pct`, which this component does not have setters for.
- `onReset: () => void` - called from `ExportProgress`'s "Try again"/"Export again" buttons, to clear a stale `done`/`error` outcome and return to the settings form on the NEXT render (this component does not close itself; the caller clears the state that `showProgress` depends on).

### Implementation

Reuses the app-wide `.e-modal-scrim`/`.e-modal` pattern (`ConfirmDialog`'s styling) with a wider `.e-export-modal` variant. Four controls: a `Picker` each for resolution (`source`/`p720`/`p1080`/`p1440`/`p2160`), frame rate (`source`/`f30`/`f60`), and format (`mp4`/`webm`/`gif`); a `Slider` (18-28, step 1) for `quality_crf`. The quality slider is `disabled` and annotated "Not used for GIF" when `format === "gif"` (GIF quality comes from the palette filter, not a CRF knob - see `encode::ffmpeg_args::export_args`). The draft starts at `DEFAULT_EXPORT_SETTINGS` (mirrors the Rust `ExportSettings::default()` - Source/60fps/CRF 24/MP4, i.e. today's export unchanged) every time the component mounts.

### Used by

- `Editor` (`src/editor/Editor.tsx`) - rendered once near `moveOffDialog`; `TopBar`'s `onOpenExport` sets the `open` state, and `onExport` wires to `exportProject(folder, settings)` after resetting `exportDone`/`exportError` and setting `exporting`/`pct`.
