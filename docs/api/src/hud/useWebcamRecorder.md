# src/hud/useWebcamRecorder.ts

Hook that wraps the browser `MediaRecorder` API to record a webcam stream into an in-memory buffer and return the encoded bytes as a plain number array suitable for Tauri IPC transfer. No effects are set up; the caller drives the lifecycle via `start` and `stop`.

## useWebcamRecorder

```ts
export function useWebcamRecorder(): {
  start: (stream: MediaStream | null) => void;
  stop: () => Promise<number[] | null>;
}
```

Returns a `{ start, stop }` pair for controlling a `video/webm` `MediaRecorder` session.

### Returns

The hook itself returns synchronously and has no state or effects. It exposes two functions via stable refs:

#### start

```ts
start(stream: MediaStream | null): void
```

Begins recording from `stream`.

- `stream: MediaStream | null` - the active webcam stream from `useWebcamPreview`. *Why nullable:* the caller passes `stream()` from the preview hook which returns `null` when no preview is active; `start` no-ops on null rather than forcing the caller to guard.
- Clears `chunks.current` so any previously buffered data from a prior recording is discarded.
- Creates a `MediaRecorder` with `{ mimeType: "video/webm" }`.
- Attaches `ondataavailable` to push non-empty `Blob` chunks into `chunks.current`.
- Calls `mr.start()` (requests data chunks as they become available) and stores the recorder in `rec.current`.

#### stop

```ts
stop(): Promise<number[] | null>
```

Stops recording and returns the encoded bytes.

- If `rec.current` is `null` (recording was never started or already stopped), returns `Promise.resolve(null)`.
- Otherwise: attaches `mr.onstop` to resolve a promise with a `Blob` assembled from all buffered chunks, typed as `"video/webm"`.
- Calls `mr.stop()` to flush the final chunk and fire `onstop`.
- Clears `rec.current` to `null` immediately (before awaiting) so a second `stop()` call returns `null` instead of double-stopping.
- Awaits the blob, reads it as an `ArrayBuffer`, wraps it in a `Uint8Array`, and returns `Array.from(...)` as `number[]`. *Why `number[]`:* Tauri's IPC `invoke` serializes `Uint8Array` unreliably across the boundary; a plain `number[]` serializes as a JSON array that Rust can deserialize into `Vec<u8>`.

Internal refs (not exposed):
- `rec` (`useRef<MediaRecorder | null>`) - the active recorder instance.
- `chunks` (`useRef<Blob[]>`) - accumulated data blobs from `ondataavailable`.

No cleanup effect is registered. If the component unmounts while recording, any in-flight `MediaRecorder` data is lost. The caller (`Hud.tsx`) must call `stop()` before unmounting.
