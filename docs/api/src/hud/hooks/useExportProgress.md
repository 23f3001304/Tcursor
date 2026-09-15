# src/hud/hooks/useExportProgress.ts

The HUD's three `export-*` Tauri listeners and the state they drive, split out of `Hud.tsx`. Export itself moved into the editor's `ExportDialog` (`src/editor/Editor.tsx`) long ago; what is left here is the HUD still being able to say that one failed, and to reveal what one produced.

## useExportProgress

```ts
export function useExportProgress(lastFolderRef: RefObject<string>): {
  exporting: boolean;
  pct: number;
  exportErr: string | null;
};
```

### Props

- `lastFolderRef: RefObject<string>` - the project folder the last recording stopped into, owned by `Hud` (`useRecordingFlow` writes it). Only the `export-error` branch reads it, and only to guess at a `video.mp4` worth revealing; it is a ref rather than a value so the listeners can stay mounted for the life of the component.

### Returns

- `exporting: boolean` - false except between an export starting and finishing. Nothing in the current app flow sets it true from the HUD, so in practice it is the flag `IdleCard` uses to decide whether to hide the Open Project / Preferences / Settings buttons, and it never does.
- `pct: number` - the last `export-progress` payload.
- `exportErr: string | null` - the last `export-error` payload. `Hud` renders it in the same titlebar slot as a recording error (`banner = warn ?? exportErr`). Before this existed an export failure was completely silent once the user was back on the HUD; only a best-effort `revealItemInDir` guess ran, which silently no-op'd for any project opened via Open Project.

### Behavior

One effect, mounted once, subscribing to three events:

- `export-progress` (payload `number`) - updates `pct` and clears `exportErr`. A new progress tick means an export is actively running now, so any stale prior failure message is no longer current.
- `export-done` (payload: the exported file's own absolute path, `<folder>/final.<ext>` - see `run.rs`, not just the project folder) - sets `exporting` false and calls `revealItemInDir` on it directly. No hardcoded `\final.mp4` guess, which used to be wrong for a webm or gif export.
- `export-error` (payload: the error message) - sets `exporting` false, sets `exportErr` to the message, and best-effort reveals `video.mp4` from `lastFolderRef` on top of it. Still a no-op when that ref is `""` (a project opened via Open Project), but the message renders regardless.

All three `listen` promises return unsubscribe functions, called on cleanup.

### Used by

- `src/hud/Hud.tsx`.
