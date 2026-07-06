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

Effect runs when `deviceId` or `enabled` changes:
- If `!enabled`: sets `on` to `false` and returns; no stream is acquired.
- Otherwise: sets `cancelled = false`, then calls `getUserMedia` with the appropriate video constraint.
- On success: if `cancelled` is still `false`, stores the stream in `streamRef.current`, assigns it to `elRef.current.srcObject` if the element exists, and sets `on` to `true`.
- On failure (permission denied, device busy, etc.): sets `on` to `false`.

Cleanup:
- Sets `cancelled = true` so any in-flight `getUserMedia` promise stops its tracks and does not attach to the element.
- Stops all tracks on `streamRef.current`.
- Clears `streamRef.current` to `null`.
