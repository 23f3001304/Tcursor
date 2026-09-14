# src/hud/hooks/useWebcamRecorder.ts

Hook that wraps the browser `MediaRecorder` API to stream a webcam preview stream to disk DURING recording, one chunk at a time, via the Tauri `append_webcam` command. No React state or effects; the caller drives the lifecycle via `start` and `stop`.

## WebcamStopAction

```ts
export type WebcamStopAction = "skip" | "stop";
```

The two outcomes `webcamStopAction` (below) can decide on for `stop()`'s recorder handling.

## webcamStopAction

```ts
export function webcamStopAction(state: RecordingState | null): WebcamStopAction
```

Pure decision: given a `MediaRecorder`'s `state` (or `null` when there is no active recorder at all), should `stop()` attempt to call `mr.stop()`?

### Inputs

- `state: RecordingState | null` - `rec.current?.state ?? null` at the moment `stop()` runs.

### Returns

`"skip"` when `state` is `null` or `"inactive"`; `"stop"` for `"recording"` or `"paused"`. *Why `"inactive"` skips:* `"inactive"` means the browser already auto-stopped the recorder on its own (all tracks ended - unplug/driver reset/another app grabbing the device, see `useWebcamPreview`'s `track.onended` handling) - calling `.stop()` again would throw `InvalidStateError` (task-6 (b) / finding H1) instead of doing anything useful.

### Behaviors (pinned by unit tests in `src/hud/hooks/useWebcamRecorder.test.ts`)

- `null` -> `"skip"` (no recorder at all).
- `"inactive"` -> `"skip"` (already auto-stopped).
- `"recording"` -> `"stop"`.
- `"paused"` -> `"stop"`.

### Used by

- `src/hud/hooks/useWebcamRecorder.ts` - `stop()`'s first branch.

## raceStopOrTimeout

```ts
export function raceStopOrTimeout(onDone: Promise<void>, timeoutMs: number): Promise<"stopped" | "timeout">
```

Races `onDone` against a `timeoutMs` timer. Split out of `stop()` purely so the timeout path has a test seam that doesn't need a real `MediaRecorder` or `vi.useFakeTimers()` wired through a hook render.

### Inputs

- `onDone: Promise<void>` - in production, the real `mr.onstop` promise `stop()` builds.
- `timeoutMs: number` - in production, always `4000`.

### Returns

`"stopped"` if `onDone` settles first; `"timeout"` if the timer fires first. *Why this exists at all:* `onstop` can in rare cases never fire (browser quirk, or a stream torn down from elsewhere mid-call) - H1's secondary hazard was that an unbounded wait here hangs the caller's entire Stop/Close flow forever (in `useRecordingFlow`, that means `saving` never clears and the Record button - `disabled={saving}` - is bricked until app restart).

### Behaviors (pinned by unit tests in `src/hud/hooks/useWebcamRecorder.test.ts`, the second using `vi.useFakeTimers()`/`vi.advanceTimersByTimeAsync`)

- `onDone` already resolved -> `"stopped"`.
- `onDone` never settles -> `"timeout"` once `timeoutMs` elapses.

### Used by

- `src/hud/hooks/useWebcamRecorder.ts` - `stop()`, awaited (not raced against anything further) once `mr.stop()` has been called successfully.

## useWebcamRecorder

```ts
export function useWebcamRecorder(): {
  start: (stream: MediaStream | null, dest: string, seg?: number) => void;
  stop: () => Promise<void>;
  folder: () => string;
  segment: () => number;
}
```

Returns a `{ start, stop, folder, segment }` set for controlling a `video/webm` `MediaRecorder` session that streams to `dest` (a project folder) as it records.

### Returns

The hook itself returns synchronously and has no state or effects. It exposes two functions via stable refs.

#### start

```ts
start(stream: MediaStream | null, dest: string, seg = 1): void
```

Begins recording from `stream`, streaming to `dest`.

- `stream: MediaStream | null` - the active webcam stream from `useWebcamPreview`. *Why nullable:* the caller passes `cam.stream()` from the preview hook, which returns `null` when no preview is active; `start` no-ops on `null` rather than forcing the caller to guard.
- `dest: string` - the project folder `append_webcam` writes chunks into (the folder `start_recording` just returned).
- `seg: number` (default `1`) - which webcam file this recorder's chunks belong to. A take normally only ever calls `start` with `1` (`webcam.webm`); a mid-take camera switch (`useSourceSwitch.switchCamera`) stops the running recorder and calls `start` again with `2`, `3`... so the new camera writes `webcam_<n>.webm` instead of appending a second stream's bytes onto the first one's file. *Why a second recorder at all:* a `MediaRecorder` cannot change its stream. `preprocess` merges the segments back into one `webcam.webm` before the editor opens.
- Resets `chain.current` to `Promise.resolve()` so any prior recording's flush chain can't bleed into this one.
- Creates a `MediaRecorder` with `{ mimeType: "video/webm" }`; `ondataavailable` chains each non-empty chunk through `chain.current` (`.arrayBuffer()` -> `appendWebcam(dest, bytes, seg)`, swallowing any single chunk's failure so one bad append can't break the chain for the rest of the take). Both `dest` and `seg` are read from refs at fire time, so a chunk is always written under the values its own recorder was started with.
- Calls `mr.start(1000)` (1s timeslices, so a chunk streams to disk roughly every second instead of one large in-memory blob written at Stop) and stores the recorder in `rec.current`.

#### stop

```ts
stop(): Promise<void>
```

Stops recording and waits for every queued chunk to finish writing.

- Reads and immediately clears `rec.current` to `null` - BEFORE any call that can throw, so a failed/auto-stopped recorder never lingers to wedge the next start/stop cycle.
- Calls `webcamStopAction(mr?.state ?? null)`. `"skip"` (no recorder, or already `"inactive"`) goes straight to awaiting `chain.current` and returns; no `mr.stop()` call is made in this case, because one would throw `InvalidStateError` on an already-inactive recorder (task-6 (b) / finding H1).
- Otherwise (`"stop"`): registers `mr.onstop`, calls `mr.stop()` inside a `try`/`catch` (guards a race where the tracks end between the state check above and this call - the `catch` falls through to awaiting `chain.current` and returns, since `onstop` will never fire for a throwing call), then awaits `raceStopOrTimeout(stopped, 4000)` so a browser quirk that never fires `onstop` can't hang the caller's whole Stop/Close flow forever, then awaits `chain.current` so every chunk (including the final one queued by the stop itself) has actually reached disk.

#### folder

```ts
folder(): string
```

The `dest` the last `start` was given, `""` before the first one (a take that began with the camera off never calls `start`). Read by `useSourceSwitch.switchCamera`, which needs the running take's project folder to open the next segment in it and has no other handle on it.

#### segment

```ts
segment(): number
```

The segment index the currently-running (or last) recorder was started with. `switchCamera` adds one to it for the next segment, so the numbering is the recorder's own truth rather than a second counter that could drift from it.

Internal refs (not exposed):
- `rec` (`useRef<MediaRecorder | null>`) - the active recorder instance.
- `chain` (`useRef<Promise<void>>`) - the ordered chain of in-flight `appendWebcam` calls.
- `folder` (`useRef<string>`) - the current `dest`, read by `ondataavailable` (which fires after `start` returns, so it cannot close over a stale local) and exposed through the getter above.
- `segment` (`useRef<number>`) - the current `seg`, read by `ondataavailable` for the same reason, and exposed through the getter above.

### Used by

- `src/hud/hooks/useRecordingFlow.ts` - `toggle()` calls `start` on Record; `stopCore()` calls `stop()` in parallel with `stopRecording()` via `Promise.allSettled`, so a webcam-side failure here can never prevent the screen/mic/system-audio take from being saved.
- `src/hud/hooks/useSourceSwitch.ts` - `switchCamera` calls `stop()`, then `start(newStream, folder(), segment() + 1)` once the new device is live.
