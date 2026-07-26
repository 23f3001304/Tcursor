# src/hud/hooks/useRecordingFlow.ts

Owns the record -> stop -> preprocess -> edit lifecycle, split out of `Hud.tsx` so that component only wires UI to it. Stop drops the recording UI immediately (screen finalize + webcam flush run in parallel), then runs the FULL editor-preview preprocessing pass (`preprocess_project`: proxy/thumbs/waveforms/preview-audio/edit.json) and reports its progress as `savePct`, so the editor opens onto already-built artifacts instead of opening while that work is still racing along on a detached background thread (the old `thumbs::prewarm` behavior) - THAT race was why the preview could still take a while to load right after Stop.

## useRecordingFlow

```ts
export function useRecordingFlow(deps: RecordingFlowDeps): {
  recording: boolean; paused: boolean; saving: boolean; savePct: number; err: string | null;
  toggle: () => Promise<void>; togglePause: () => Promise<void>;
}
```

### Inputs

`deps: RecordingFlowDeps`:
- `micOn: boolean`, `micId: string | null`, `displayId: string | null`, `sysOn: boolean`, `gameMode: boolean`, `camOn: boolean` - the HUD's current device/toggle selections, passed straight through to `startRecording`.
- `camStream: () => MediaStream | null` - `cam.stream` from `useWebcamPreview`; called (only if `camOn`) right after `startRecording` resolves, to start the webcam recorder against the live stream.
- `webcam: { start, stop }` - the `useWebcamRecorder()` instance; `start(stream, folder)` on Record, `stop()` on Stop (both already stream to disk via `appendWebcam` during recording, so `stop()` only has to flush the tail chunk).
- `onEdit?: (folder: string) => void` - forwarded from `Hud`'s own prop; called once preprocessing finishes (or errors), with the finished project's folder.
- `lastFolderRef: { current: string }` - a ref OWNED by the caller (`Hud`'s `lastFolder`). Written with the folder on Stop so `Hud`'s own (unrelated) `export-error` listener can still reveal it - kept out of this hook's own state since it isn't part of the recording lifecycle itself.

### Returns

- `recording` / `paused` - the two recording phases.
- `saving` - true from the moment Stop is pressed until `onEdit` is about to be called (finalize + webcam flush + the full `preprocess_project` pass).
- `savePct` - 0..100, mirrors the latest `preprocess-progress` event; reset to 0 at the start of each Stop.
- `err` - last `startRecording` failure string (cleared at the start of the next Record attempt).
- `toggle` - the Record/Stop button handler (see Implementation).
- `togglePause` - calls `pauseRecording`/`resumeRecording` and flips `paused`.

### Implementation

**Preprocess event listeners (`useEffect` on `[]`, mount-time).** Subscribes to `preprocess-progress` (updates `savePct`), `preprocess-done`, and `preprocess-error` ONCE, well before any recording can finish - the same reasoning `useEditorData`'s export-progress listener follows for exports. `preprocess-done`/`preprocess-error` both just invoke whatever resolver `toggle()`'s Stop path most recently stashed in the `preprocessDone` ref (see below); unsubscribed on unmount.

**`toggle()` - Record (`!recording`).** Clears `err`. Calls `startRecording(\`rec-${Date.now()}\`, micOn ? micId : null, displayId, sysOn, gameMode)`. On failure, sets `err` and returns. On success, starts the webcam recorder (`webcam.start(camStream(), folder)`) if `camOn`, then sets `recording = true`.

**`toggle()` - Stop (`recording`).** Immediately sets `recording = false`, `paused = false`, `saving = true`, `savePct = 0` (so the UI reacts before any awaiting happens). Then:
1. `Promise.all([stopRecording(), webcam.stop()])` - screen finalize and the webcam's tail-chunk flush run in parallel (both quick; the webcam already streamed to disk during recording).
2. Writes the returned folder into `lastFolderRef.current`.
3. Awaits a `Promise` whose resolver is stashed in the `preprocessDone` ref, then calls `preprocessProject(folder)` - resolved by the mount-time listener above on `preprocess-done`/`preprocess-error`, OR immediately (via `.catch`) if the `preprocessProject` invoke itself rejects, so a failure to even START preprocessing can't hang "Saving..." forever (the editor's own lazy `ensure_*` fallback still covers a project preprocessing never ran for).
4. Calls `onEdit?.(folder)`.
5. `finally`: `saving = false`, clears the `preprocessDone` ref.

Any error thrown by step 1 (finalize failure) is caught and set into `err`; `onEdit` is not called in that case.

### Used by

`Hud` (`src/hud/Hud.tsx`) - the sole caller; destructures the full return value and renders `saving`/`savePct` as a progress pill alongside the existing `exporting`/`pct` one.
