# src/hud/hooks/useDevices.ts

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

- A `refresh` function fires `Promise.all([listDisplays(), listAudioInputs()])`, then calls `setDisplays`, `setMics`, and `setSel(resolveSelection(prev, d, m))` (see `selectDevices.md`) in one synchronous batch. `resolveSelection` keeps the current pick if it is still present in the fresh list, or falls back to the first available device - on the very first call `prev` is `{ displayId: null, micId: null }`, which resolves identically to `pickDefaults`.
- `refresh()` is called once inside a `useEffect` with an empty dependency array (mount), AND registered as a `navigator.mediaDevices` `"devicechange"` listener (task-6 (d)) - so plugging/unplugging a display or mic re-enumerates and swaps out a selection that just went stale, instead of leaving `sel` pointing at a device that will only fail once Record is pressed (surfaced today via a `record-warning` event from `spawn_mic_thread`/`spawn_system_thread`, but only at that later point).
- A `cancelled` flag (closed over by `refresh`) guards the mount-time call's `setState`s in case the component unmounts before the first `Promise.all` resolves; the `devicechange` listener is removed on cleanup.
- Errors from IPC are not caught here; Tauri surfaces them as rejected promises that will be unhandled. The caller can wrap the hook in an error boundary if needed.

### Used by

- `src/hud/Hud.tsx` - destructures all four fields at the root component level
