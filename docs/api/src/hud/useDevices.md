# src/hud/useDevices.ts

Hook that fetches the available displays and audio inputs from Tauri on mount, then exposes them alongside a user-controllable selection. The single integration point between IPC device enumeration and the HUD's device picker UI.

## useDevices

```ts
export function useDevices(): {
  displays: DisplayInfo[];
  mics: AudioInfo[];
  sel: DeviceState;
  setSel: React.Dispatch<React.SetStateAction<DeviceState>>;
}
```

Loads display and microphone lists over IPC, initializes default selections, and returns both the lists and live selection state.

### Returns

- `displays: DisplayInfo[]` - list of monitors returned by `listDisplays`, starts empty and populates after the IPC call resolves.
- `mics: AudioInfo[]` - list of microphones returned by `listAudioInputs`, same lifecycle.
- `sel: DeviceState` - currently selected `{ displayId, micId }`. Starts as `{ displayId: null, micId: null }` and is updated to `pickDefaults(displays, mics)` once both IPC calls resolve. Set to `null` fields whenever no device of that type is available.
- `setSel: Dispatch<SetStateAction<DeviceState>>` - state setter exposed so the caller can update the selection when the user picks a different device from the UI.

### Behavior

- Fires a single `Promise.all([listDisplays(), listAudioInputs()])` inside a `useEffect` with an empty dependency array, so it runs exactly once on mount.
- On resolution, calls `setDisplays`, `setMics`, and `setSel(pickDefaults(d, m))` in one synchronous batch.
- No cleanup: the IPC calls are fire-and-forget reads; there are no subscriptions or streams to release.
- Errors from IPC are not caught here; Tauri surfaces them as rejected promises that will be unhandled. The caller can wrap the hook in an error boundary if needed.

### Used by

- `src/hud/Hud.tsx` - destructures all four fields at the root component level
