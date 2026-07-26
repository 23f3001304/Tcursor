# src/hud/devices/selectDevices.ts

Pure data types and a default-selection helper for the display and audio input device lists, plus a label-parsing helper for the capture-target picker. No React, no side effects - just interfaces and two functions. Consumed by `useDevices` to initialize selections after the IPC lists resolve, and by `TargetPicker` to render each target's row.

## DisplayInfo

```ts
export interface DisplayInfo { id: string; label: string; kind?: string }
```

Descriptor for one capture target (a monitor or an app window) returned by the Tauri IPC `list_displays` command.

- `id: string` - opaque target identifier: `"display:N"` for a monitor (`N` its enumeration index) or `"window:0x…"` for an app window (its HWND in hex). *Why opaque:* it round-trips straight into `startRecording`'s `targetId` argument; nothing on the frontend parses it.
- `label: string` - human-readable string shown in the picker, e.g. `"Display 1: \\.\DISPLAY1 (Primary)"` or `"App: Notepad"`. The backend bakes a trailing `(Primary)` or `(WxH)` onto display labels - see `parseTarget`, which splits that back out for display.
- `kind?: string` - `"display"` or `"window"`. *Why optional:* older cached/mocked data may omit it; callers that group by kind treat a missing value as `"display"` (i.e. `kind !== "window"`).

### Used by

- `src/hud/hooks/useDevices.ts` - stores the list returned by `listDisplays`
- `src/hud/devices/TargetPicker.tsx` - groups the list into displays vs. windows and renders each row
- `src/hud/Hud.tsx` - passes the list to `TargetPicker`

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
export interface DeviceState { displayId: string | null; micId: string | null }
```

Holds the currently selected display/window target and microphone IDs. Either field is `null` when no device of that type is available or has not yet been chosen.

- `displayId: string | null` - the selected `DisplayInfo.id` (a `target_id`), or `null`. *Why nullable:* the IPC call may return an empty list (no capture targets detected), in which case `null` signals "no valid selection".
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

## parseTarget

```ts
export function parseTarget(t: DisplayInfo, index: number): { title: string; resolution: string | null; primary: boolean }
```

Splits a `DisplayInfo.label` into a clean `title` plus the pieces `TargetPicker` renders as a separate resolution sub-line and "Primary" badge, instead of showing the backend's raw parenthetical text verbatim.

### Inputs

- `t: DisplayInfo` - one target from a `listDisplays()` result.
- `index: number` - `t`'s position in the raw (unfiltered) `listDisplays()` array. *Why the caller must pass it:* the backend only ever marks the very first monitor as `(Primary)`, and displays always enumerate before windows, so raw index 0 is the sole source of truth for `primary` - a target's position within a post-filtering subgroup would give the wrong answer once windows are involved.

### Returns

- `title: string` - `t.label` with a trailing `" (Primary)"` or `" (WxH)"` stripped, or `t.label` unchanged if it matches neither pattern (e.g. a window's `"App: <title>"`, or a display whose resolution the backend couldn't read).
- `resolution: string | null` - the extracted `"WxH"` substring, or `null` when the label had no resolution suffix (the primary display, or one the backend can't measure).
- `primary: boolean` - `index === 0 && t.kind !== "window"`. Always `false` for windows, even a hypothetical one at index 0.

### Used by

- `src/hud/devices/TargetPicker.tsx` - calls it per row to render the title/resolution/badge
