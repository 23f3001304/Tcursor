# src/hud/hooks/useRecordingFlow.ts

Owns the record -> stop -> preprocess -> edit lifecycle, split out of `Hud.tsx` so that component only wires UI to it. Stop drops the recording UI immediately (screen finalize + webcam flush run in parallel), then runs the FULL editor-preview preprocessing pass (`preprocess_project`: proxy/thumbs/waveforms/preview-audio/edit.json) and reports its progress as `savePct`, so the editor opens onto already-built artifacts instead of opening while that work is still racing along on a detached background thread (the old `thumbs::prewarm` behavior) - THAT race was why the preview could still take a while to load right after Stop.

## StopOutcome

```ts
export type StopOutcome = { folder: string; camWarn?: string } | { err: string };
```

The result of combining the screen-stop and webcam-stop `Promise.allSettled` results (`settleStop`, below). A discriminated union rather than one object with optional fields, so a caller checking `"err" in outcome` gets a `folder`-less type in the error branch - there is nothing to hand off when the screen stop itself failed.

- `{ folder, camWarn? }` - the screen (mic/system-audio included) stop succeeded. `camWarn` is present only when the webcam side rejected - a soft warning, never a reason to discard the folder.
- `{ err }` - the screen stop itself rejected; nothing to hand off, and any webcam-side outcome is irrelevant.

## settleStop

```ts
export function settleStop(
  screen: PromiseSettledResult<{ folder: string; frames: number }>,
  cam: PromiseSettledResult<void>,
): StopOutcome
```

Pure decision function: given the two `Promise.allSettled` results from stopping the Rust recording and flushing the webcam recorder, decides the single `StopOutcome`. This is the exact combinator semantics whose WRONG version - `Promise.all`, which fails the whole stop on ANY rejection - was bug H1 (a webcam-side `MediaRecorder.stop()` throw used to make a fully-successful screen/mic/system-audio finalize look like a failed Stop, and the editor never opened).

### Inputs

- `screen: PromiseSettledResult<{ folder: string; frames: number }>` - the settled result of `stopRecording()`.
- `cam: PromiseSettledResult<void>` - the settled result of `webcam.stop()`.

### Returns

- `screen.status === "rejected"` -> `{ err: "Recording failed: " + screen.reason }`, unconditionally - a webcam-side outcome (rejected or not) is not consulted at all once the screen itself failed.
- Otherwise -> `{ folder: screen.value.folder, camWarn }`, where `camWarn` is `"Webcam track didn't finish — video saved without the camera overlay."` if `cam.status === "rejected"`, else `undefined`.

### Behaviors (pinned by unit tests in `src/hud/hooks/useRecordingFlow.test.ts`)

- Both settle fine -> the folder, `camWarn: undefined`.
- Only the webcam side rejected -> the folder PLUS a `camWarn` (fix round 1, item 2 - this used to be a bare `console.warn`, invisible in the running app; `stopCore` now routes `camWarn` through `setErr` so it reaches the same banner every other take-level message uses).
- Only the screen side rejected -> `{ err }`, even though the webcam side succeeded.
- Both rejected -> still just `{ err }` from the screen side; the camera warning is dropped, not merged in.

### Used by

- `src/hud/hooks/useRecordingFlow.ts` - `stopCore()` is the sole caller.

## useRecordingFlow

```ts
export function useRecordingFlow(deps: RecordingFlowDeps): {
  recording: boolean; paused: boolean; saving: boolean; savePct: number; err: string | null;
  toggle: () => Promise<void>; togglePause: () => Promise<void>; stopForClose: () => Promise<void>;
}
```

### Inputs

`deps: RecordingFlowDeps`:
- `micOn: boolean`, `micId: string | null`, `displayId: string | null`, `sysOn: boolean`, `gameMode: boolean`, `camOn: boolean` - the HUD's current device/toggle selections, passed straight through to `startRecording`.
- `camStream: () => MediaStream | null` - `Hud.tsx` passes `() => camStreamRef.current()`, a ref indirection onto `cam.stream` from `useWebcamPreview` (needed since `Hud.tsx`'s `cam` is declared AFTER this hook, so it can't pass `cam.stream` directly - see `Hud.md`'s `camStreamRef` note); called (only if `camOn`) right after `startRecording` resolves, to start the webcam recorder against the live stream.
- `webcam: { start, stop }` - the `useWebcamRecorder()` instance; `start(stream, folder)` on Record, `stop()` on Stop (both already stream to disk via `appendWebcam` during recording, so `stop()` only has to flush the tail chunk).
- `onEdit?: (folder: string) => Promise<void>` - forwarded from `Hud`'s own prop; called once preprocessing finishes (or errors), with the finished project's folder, and AWAITED via `./handOff.ts` (fix round 1, item 2, see its own doc) before `finish` decides whether to reset `saving`. NOT called by `stopForClose` - see below.
- `lastFolderRef: { current: string }` - a ref OWNED by the caller (`Hud`'s `lastFolder`). Written with the folder on Stop so `Hud`'s own (unrelated) `export-error` listener can still reveal it - kept out of this hook's own state since it isn't part of the recording lifecycle itself.

### Returns

- `recording` / `paused` - the two recording phases.
- `saving` - true from the moment Stop (or a graceful close) is triggered until the take is either fully handed to the editor or fully abandoned (finalize + webcam flush, plus the full `preprocess_project` pass for `finish`; finalize + webcam flush only for `stopForClose`).
- `savePct` - 0..100, mirrors the latest `preprocess-progress` event; reset to 0 at the start of each stop.
- `err` - the last thing the user needs to know about the take: a start/stop failure (`"Recording failed: …"`), a backend `record-warning` (an audio input that would not open - the take is still running), or the `record-ended-early` reason. Cleared at the start of the next Record attempt. `Hud.tsx` renders it verbatim after a warning glyph, so each message carries its own subject.
- `toggle` - the Record/Stop button handler (see Implementation); the Stop half is `finish`, which the backend can also trigger.
- `togglePause` - calls `pauseRecording`/`resumeRecording` and flips `paused`.
- `stopForClose` - the graceful-close stop (task-6 (a) / ruling R6). `Hud`'s window Close button awaits this before actually calling `window.close()`, so closing mid-recording finalizes the take instead of destroying it.

### Implementation

**Event listeners (`useEffect` on `[]`, mount-time).** Subscribes ONCE, well before any recording can finish - the same reasoning `useEditorData`'s export-progress listener follows for exports; all unsubscribed on unmount.
- `preprocess-progress` updates `savePct`; `preprocess-done` / `preprocess-error` both invoke whatever resolver the Stop path most recently stashed in the `preprocessDone` ref (see below).
- `record-warning` sets `err` and nothing else - the take is degraded (typically no narration, because an audio input would not open) but still running, and the whole point is that the user learns now rather than when the editor opens.
- `record-ended-early` sets `err` AND calls `endEarly.current()`, which runs the same `finish()` a user Stop runs. *Why run the full stop:* the OS already ended the capture (recorded window closed, display unplugged), so the video stopped there whatever the HUD does; the only wrong outcome is a HUD that keeps counting while the mic records narration over footage that no longer exists. Finalizing normally means the take is saved and opens in the editor like any other.

**`endEarly` / `finishing` / `starting` refs.** `endEarly` is re-pointed on every render, so the mount-time listener always calls the CURRENT `finish` with the current `recording`/`saving` values without ever re-subscribing; it no-ops unless a recording is actually in progress. `finishing` is an in-flight latch shared by `finish` AND `stopForClose`: only one of Stop, a backend-initiated end, or a window-close can ever own the actual `stop_recording` call for a given take - running the finalize twice would double-invoke it (the second returning "not recording"). `starting` is the analogous re-entrancy guard for `startRecording` (task-6 (f) / finding M2): it stays slow (settings snapshot, spawning every tracker/thread, building the encoder) with `recording` still `false` for a second or two, so without this a fast double-click fired a second `start_recording` that Rust rejected with "already recording" - painting a permanent-looking failure banner over a take that was actually running fine.

**`stopCore()` - the shared stop primitive.** Awaits `Promise.allSettled([stopRecording(), webcam.stop()])`, then feeds both results through `settleStop` (above) to get one `StopOutcome`. `"err" in outcome` sets `err` and returns `null` - both `finish` and `stopForClose` treat that as "nothing to hand off", since a `stop_recording` error no longer implies a lost take (the Rust side writes the project folder before reporting a video-finalize failure - see `recorder_stop.md`), but neither the editor nor a close should proceed as if it fully succeeded. Otherwise, `outcome.camWarn` (if present) is routed through the same `setErr` the rest of this hook uses - a webcam-side rejection (e.g. a `MediaRecorder` `InvalidStateError` from a device that died mid-take - see `useWebcamRecorder.md`) is now VISIBLE, not just logged; the take is still fully there without its PiP tail. Writes `outcome.folder` into `lastFolderRef.current` and returns it.

**`toggle()` - Record (`!recording`).** Bails immediately if `starting.current` is already set. Clears `err`. Calls `startRecording(\`rec-${Date.now()}\`, micOn ? micId : null, displayId, sysOn, gameMode)`. On success, starts the webcam recorder (`webcam.start(camStream(), folder)`) if `camOn`, then sets `recording = true` - both inside the SAME `try` as the `startRecording` call, so a `webcam.start` throw (task-6 (e) / finding M1) is caught too: the `catch` sets `err` and, since `folder` was already assigned (proving Rust IS actively recording with nothing on screen to show for it), calls `stopRecording()` to clean up the orphaned backend session rather than leaving it running forever with an idle HUD and no way back in except Close (which used to be Critical finding C1).

**`finish()` - the Stop path** (called by `toggle()` when recording, and by the `record-ended-early` listener). Returns immediately if `finishing` is already set. Otherwise sets `recording = false`, `paused = false`, `saving = true`, `savePct = 0` (so the UI reacts before any awaiting happens), then:
1. Calls `stopCore()`. Returns early (leaving `err` as `stopCore` set it, and the local `resetSaving` at its default `true`) if it returned `null`.
2. Awaits a `Promise` whose resolver is stashed in the `preprocessDone` ref, then calls `preprocessProject(folder)` - resolved by the mount-time listener above on `preprocess-done`/`preprocess-error`, OR immediately (via `.catch`) if the `preprocessProject` invoke itself rejects, so a failure to even START preprocessing can't hang "Saving..." forever (the editor's own lazy `ensure_*` fallback still covers a project preprocessing never ran for).
3. Sets `resetSaving = await handOff(onEdit, folder)` (`./handOff.ts` - fix round 1, item 2, see its own doc for why this is awaited rather than fired-and-forgotten). A rejection here propagates out of the `try`, leaving `resetSaving` at its default `true`.
4. `finally`: resets `saving = false` ONLY `if (resetSaving)` - i.e. every path except a successful handoff (stopCore failure, a thrown/rejected `handOff`, or no `onEdit` at all still reset it; a successful handoff deliberately leaves `saving` true, since `Hud` is about to unmount with the editor already open). Clears the `preprocessDone` ref AND the `finishing` latch unconditionally either way.

**`stopForClose()` - the graceful-close path** (task-6 (a) / R6). A trimmed `finish()`: same `recording`/`saving`/`finishing` bookkeeping and the same `stopCore()` call, but no `preprocessProject` and no `onEdit` - there is no reason to build editor-only artifacts (proxy/thumbnails/waveforms) for a project the window is about to close on top of, and the folder is still fully valid without them (the editor's own lazy `ensure_*` fallback covers it if it's ever reopened). Returns immediately (no-op) if neither `recording` nor `saving` is true, or if `finishing` is already set (a Stop or an earlier close already owns this take). `Hud`'s Close button (`handleClose`) awaits this and then calls `window.close()` unconditionally - by the time this resolves, the Rust `Recorder` is no longer busy, so the Rust-side `CloseRequested` guard (`lib.rs`/`close_guard.rs`) sees nothing to intercept and the close proceeds normally. That Rust guard is the actual backstop against data loss (it does not depend on this function ever running - the OS close button and a wedged renderer both reach it directly); this function exists for the better common-case UX (a visible "Saving…" state instead of the window just hanging while Rust works invisibly).

### Used by

`Hud` (`src/hud/Hud.tsx`) - the sole caller; destructures the full return value, renders `saving`/`savePct` as a progress pill alongside the existing `exporting`/`pct` one, and calls `stopForClose` from its Close button handler.
