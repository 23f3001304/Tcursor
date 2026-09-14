# src/hud/hooks/useSourceSwitch.ts

The three mid-take source switches behind the take pill's Sources sheet (design: `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`, plan Task C). Each one applies to a RUNNING take *and* to the HUD's own selection, so the sheet's pickers behave the same before and during a take, and the idle card always shows what the take is actually using.

The hook owns one piece of state, `err`. A switch that fails never ends the take: the message lands there, `Hud` merges it with `useRecordingFlow`'s own `err` into the take pill's existing warning slot, and the recording carries on with whatever it already had.

## pollFor

```ts
export async function pollFor<T>(get: () => T, ok: (v: T) => boolean, timeoutMs: number, stepMs = 100): Promise<T | null>
```

Polls `get` every `stepMs` until `ok` accepts what it returns, resolving that value; resolves `null` once `timeoutMs` is spent.

### Inputs

- `get: () => T` - read the thing being waited on. In production, `useWebcamPreview`'s `stream()` getter.
- `ok: (v: T) => boolean` - has it arrived? In production, "non-null and a different `MediaStream.id` than the one that was live before the switch".
- `timeoutMs: number` - total budget. In production, `CAMERA_WAIT_MS`.
- `stepMs: number` (default `100`) - poll interval.

### Returns

The first accepted value, or `null` when the budget runs out.

*Why polling at all:* `useWebcamPreview` re-acquires the device inside an effect, not through a promise the caller can await. The only signal that the newly picked camera is live is its stream appearing in that hook's ref, one render later. Exported and pure so the give-up path is unit-tested with fake timers rather than an unplugged webcam.

### Behaviors (pinned by `src/hud/hooks/useSourceSwitch.test.ts`)

- A value that is already acceptable returns without waiting at all.
- A value that only becomes acceptable later keeps being polled until it is.
- A value that never becomes acceptable resolves `null` once the budget is spent.

## CAMERA_WAIT_MS

```ts
export const CAMERA_WAIT_MS = 5000
```

How long a newly picked camera has to produce a stream before `switchCamera` gives up and says so. Deliberately generous: a cold USB camera can take a second or two to hand over its first frame, and the cost of waiting is a slightly later second segment, while the cost of giving up too early is a take that silently stops recording the camera.

## SourceSwitchDeps

```ts
export interface SourceSwitchDeps {
  recording: boolean;
  camOn: boolean; micOn: boolean;
  camId: string | null; camStream: () => MediaStream | null;
  webcam: {
    start: (stream: MediaStream | null, folder: string, segment?: number) => void;
    stop: () => Promise<void>; folder: () => string; segment: () => number;
  };
  setCamId: (id: string | null) => void;
  setMicId: (id: string) => void;
  setDisplayId: (id: string) => void;
}
```

Everything the hook needs, all of it `Hud`'s own state so nothing is duplicated.

- `recording` - while false every handler is a plain selection change (the backend commands all reject with "not recording" anyway).
- `camOn` / `micOn` - the take's own toggles. A source that is OFF for this take is only pre-selected for the NEXT one, never switched live: there is no first segment on disk for a merge to extend, so a segment written now would be a file nothing reads.
- `camId` - the current pick, so re-picking the same camera is a no-op instead of a stop-and-wait for a stream that never changes.
- `camStream` - `useWebcamPreview`'s stream getter, polled through `pollFor`.
- `webcam` - `useWebcamRecorder`'s returned API, including its `folder()` and `segment()` getters.
- `setCamId` / `setMicId` / `setDisplayId` - apply the pick to the HUD's selection once the switch has actually taken.

## useSourceSwitch

```ts
export function useSourceSwitch(d: SourceSwitchDeps): {
  err: string | null;
  clearErr: () => void;
  switchCamera: (id: string | null) => Promise<void>;
  switchMic: (id: string) => Promise<void>;
  switchDisplay: (id: string) => Promise<void>;
}
```

### Returns

- `err` - the last failed switch's message, or `null`. `Hud` shows it through the take pill's warning glyph.
- `clearErr` - called when the Sources sheet is opened, so a stale failure does not outlive the sheet that caused it.

#### switchCamera

```ts
switchCamera(id: string | null): Promise<void>
```

The only switch the webview itself owns, and the only one that is more than one IPC call, because a `MediaRecorder` cannot change its stream:

1. Re-picking the currently selected camera returns immediately (nothing would change, and the wait below would time out on a stream that never gets replaced).
2. Idle, or camera off for this take: just `setCamId(id)` and return.
3. `webcam.folder()` and `webcam.segment() + 1` are read BEFORE the stop, since the recorder is the only holder of both.
4. `await webcam.stop()` - the running segment's tail reaches disk before anything else happens.
5. `setCamId(id)` - `useWebcamPreview` tears the old device down and acquires the new one through its effect.
6. `id === null` (the camera going off mid-take) stops here: the recorder is flushed and nothing is marked, because there is no new segment to place.
7. `pollFor` waits up to `CAMERA_WAIT_MS` for a stream with an id different from the one that was live. A timeout sets `err` and leaves the take running without the camera.
8. `markWebcamSegment(next)` stamps where the new segment starts on the recording clock. A rejection sets `err` and, deliberately, does NOT start the new recorder: an unstamped `webcam_<n>.webm` is a file the merge would never place.
9. `webcam.start(stream, dest, next)` - the second recorder opens `webcam_<n>.webm`.

#### switchMic

```ts
switchMic(id: string): Promise<void>
```

Idle, or mic off for this take: just `setMicId(id)`. Otherwise `switchMic(id || null)` (`src/lib/ipc.ts`), which has Rust finalize the running WAV and start `mic_<n>.wav` on the new device; the selection is applied only if that succeeds, so a failed switch leaves the HUD showing the device that is actually recording.

#### switchDisplay

```ts
switchDisplay(id: string): Promise<void>
```

Idle: just `setDisplayId(id)`. Otherwise `switchDisplay(id)` (`src/lib/ipc.ts`), which has Rust restart the capture on the new target into the same encoder canvas and remap mouse coordinates at capture time. Same "apply only on success" rule as the mic.

### Used by

- `src/hud/Hud.tsx` - the three handlers are what the Sources sheet's pickers call; `Hud` closes the sheet in the same click.
