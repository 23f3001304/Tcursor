# src/hud/useCameraDevices.ts

Hook and supporting interface for enumerating available video input devices via the browser MediaDevices API. Labels are initially empty until camera permission is granted, so a refresh mechanism is built in.

## CameraInfo

```ts
export interface CameraInfo { id: string; label: string }
```

Minimal descriptor for a single enumerated camera device.

- `id: string` - the `deviceId` string from the MediaDevices API. *Why string:* browser device IDs are opaque strings; preserving the exact value is required to construct `getUserMedia` constraints.
- `label: string` - human-readable camera name, or `"Camera N"` if the label is empty. *Why a fallback:* `enumerateDevices` returns empty labels before camera permission is granted; the index-based fallback prevents blank entries in the picker.

### Used by

- `src/hud/useCameraDevices.ts` - returned as the state array type
- `src/hud/Hud.tsx` - camera selection UI

## useCameraDevices

```ts
export function useCameraDevices(refreshKey: number): CameraInfo[]
```

Enumerates `videoinput` devices and returns them as a `CameraInfo[]` state array.

### Arguments

- `refreshKey: number` - an opaque counter. *Why:* the browser only populates device labels after camera permission is granted; callers should increment this value after the preview stream starts (which implicitly grants permission) so that real device names replace the `"Camera N"` fallbacks.

### Returns

`CameraInfo[]` - starts empty on mount, updated asynchronously once `enumerateDevices()` resolves. The array is freshly computed on each effect run.

### Behavior

- Calls `navigator.mediaDevices?.enumerateDevices()` inside a `useEffect` that depends on `[refreshKey]`. Runs on mount and again whenever `refreshKey` changes.
- Filters the device list to `kind === "videoinput"` entries.
- Assigns the label `"Camera ${i + 1}"` (1-indexed) when `d.label` is empty.
- Errors from `enumerateDevices` are silently caught and the state is left unchanged.
- No cleanup is needed because there is no stream or listener to release.
