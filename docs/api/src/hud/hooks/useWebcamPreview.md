# src/hud/hooks/useWebcamPreview.ts

Hook that manages a live webcam preview stream for a chosen device, keeping the stream alive across `<video>` element remounts. A callback ref pattern ensures the preview is re-attached whenever the element re-mounts, avoiding the black-frame problem that occurs when a `<video>` with no `srcObject` is inserted into the DOM.

## useWebcamPreview

```ts
export function useWebcamPreview(
  deviceId: string | null,
  enabled: boolean
): { ref: (el: HTMLVideoElement | null) => void; stream: () => MediaStream | null; on: boolean }
```

Acquires a `getUserMedia` video stream for the given device and attaches it to a `<video>` element. Stream lifetime is controlled by `enabled`.

### Arguments

- `deviceId: string | null` - the `deviceId` of the camera to preview. `null` passes `{ video: true }` to `getUserMedia`, letting the browser pick the default device. *Why exact-match constraint:* passing `{ deviceId: { exact: id } }` prevents the browser from silently falling back to a different device when the requested one is busy.
- `enabled: boolean` - when `false`, the stream is not acquired (or is torn down if already live) and `on` becomes `false`. *Why a boolean rather than a stream:* the hook owns the stream lifecycle so callers do not need to manage `getUserMedia` themselves.

### Returns

- `ref: (el: HTMLVideoElement | null) => void` - callback ref to assign to the `<video>` element's `ref` prop. Stores `el` in `elRef.current` and immediately sets `el.srcObject` to the current stream if one exists. *Why a callback ref rather than `useRef`:* callback refs fire each time the element mounts, so the stream is always re-attached after the `<video>` is recreated (e.g. when a settings panel opens and closes).
- `stream: () => MediaStream | null` - getter function that returns `streamRef.current`. Intended for passing to `useWebcamRecorder.start()` when recording begins.
- `on: boolean` - `true` when a live stream is attached, `false` otherwise. Used to show/hide the preview UI and to drive `useCameraDevices`'s `refreshKey`.

### Behavior

Acquire effect runs when `deviceId`, `enabled`, or the internal `retry` counter changes:
- If `!enabled`: sets `on` to `false` and returns; no stream is acquired.
- Otherwise: sets `cancelled = false`, then calls `getUserMedia` with the appropriate video constraint.
- On success: if `cancelled` is still `false`, stores the stream in `streamRef.current`, assigns it to `elRef.current.srcObject` if the element exists, sets `on` to `true`, and attaches `track.onended` to every track (sets `on` back to `false` if the device dies mid-preview - M4).
- On failure (permission denied, device busy, etc.): sets `on` to `false`.

Cleanup:
- Sets `cancelled = true` so any in-flight `getUserMedia` promise stops its tracks and does not attach to the element.
- Stops all tracks on `streamRef.current`.
- Clears `streamRef.current` to `null`.

**Recovery effect (`devicechange`, M4).** A second effect (deps `[enabled, on]`) adds a `navigator.mediaDevices` `devicechange` listener while `enabled`; on any device change, if `on` is currently `false` it bumps an internal `retry` counter, which is in the acquire effect's dependency array and so re-runs it. *Why needed at all:* re-plugging a camera changes neither `deviceId` nor `enabled`, so without this the acquire effect never re-runs on its own and a hot-unplugged preview stayed dead until the app restarted. Only retries while a preview is actually wanted (`enabled`) and currently isn't showing one (`!on`), so an unrelated device event (e.g. a mic being plugged in) while the camera is already live is a no-op.
