# src/hud/devices/selectDevices.ts

Pure data types and a default-selection helper for the display and audio input device lists. No React, no side effects - just interfaces and one function. Consumed by `useDevices` to initialize selections after the IPC lists resolve.

## DisplayInfo

```ts
export interface DisplayInfo { id: number; label: string }
```

Minimal descriptor for a monitor returned by the Tauri IPC `listDisplays` command.

- `id: number` - numeric display identifier as assigned by the OS. *Why numeric:* Tauri's display enumeration returns integer IDs; keeping the same type avoids a conversion that would lose information for multi-display setups.
- `label: string` - human-readable display name shown in the picker (e.g. `"Built-in Retina Display"`).

### Used by

- `src/hud/hooks/useDevices.ts` - stores the list returned by `listDisplays`
- `src/hud/Hud.tsx` - passes the list to the device selector UI

## AudioInfo

```ts
export interface AudioInfo { id: string; label: string }
```

Minimal descriptor for a microphone returned by the Tauri IPC `listAudioInputs` command.

- `id: string` - string device identifier. *Why string:* audio device IDs are OS-level strings (e.g. CoreAudio UIDs on macOS); using a string avoids implicit numeric coercion.
- `label: string` - human-readable mic name shown in the picker.

### Used by

- `src/hud/hooks/useDevices.ts` - stores the list returned by `listAudioInputs`
- `src/hud/Hud.tsx` - passes the list to the device selector UI

## DeviceState

```ts
export interface DeviceState { displayId: number | null; micId: string | null }
```

Holds the currently selected display and microphone IDs. Either field is `null` when no device of that type is available or has not yet been chosen.

- `displayId: number | null` - the selected `DisplayInfo.id`, or `null`. *Why nullable:* the IPC call may return an empty list (no external monitors detected), in which case `null` signals "no valid selection".
- `micId: string | null` - the selected `AudioInfo.id`, or `null`. Same rationale as `displayId`.

### Used by

- `src/hud/hooks/useDevices.ts` - the `sel` state is typed as `DeviceState`
- `src/hud/Hud.tsx` - reads `sel.displayId` and `sel.micId` when starting a recording

## pickDefaults

```ts
export function pickDefaults(displays: DisplayInfo[], mics: AudioInfo[]): DeviceState
```

Returns an initial `DeviceState` by selecting the first element from each list, or `null` when a list is empty.

### Inputs

- `displays: DisplayInfo[]` - ordered list from `listDisplays`. *Why first element:* the first display is the primary/main display on all platforms.
- `mics: AudioInfo[]` - ordered list from `listAudioInputs`. *Why first element:* the first mic is the OS default input device.

### Returns

`DeviceState` with `displayId = displays[0]?.id ?? null` and `micId = mics[0]?.id ?? null`. Safe to call with empty arrays; optional chaining ensures no index-out-of-bounds.

### Behaviors (pinned by unit tests in `src/hud/devices/selectDevices.test.ts`)

- With one display and two mics, returns the first display id and the first mic id (`selects the first display and mic when present`).
- With one display and an empty mic list, returns `micId: null` (`leaves mic null when no inputs`).
