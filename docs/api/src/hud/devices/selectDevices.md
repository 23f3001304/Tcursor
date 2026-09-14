# src/hud/devices/selectDevices.ts

Pure data types and a default-selection helper for the display and audio input device lists, plus label-cleanup helpers for the capture-target and device pickers. No React, no side effects - just interfaces and functions. Consumed by `useDevices` to initialize selections after the IPC lists resolve, by `Hud` to clean up mic/camera dropdown labels, and by `TargetSheet` and `IdleCard` to render each target (and drop the app's own window from the list).

## DisplayInfo

```ts
export interface DisplayInfo { id: string; label: string; kind?: string }
```

Descriptor for one capture target (a monitor or an app window) returned by the Tauri IPC `list_displays` command.

- `id: string` - opaque target identifier: `"display:N"` for a monitor (`N` its enumeration index) or `"window:0x…"` for an app window (its HWND in hex). *Why opaque:* it round-trips straight into `startRecording`'s `targetId` argument; nothing on the frontend parses it.
- `label: string` - human-readable string shown in the picker, e.g. `"Display 1: \\.\DISPLAY1 (2560x1440, Primary)"` or `"App: Notepad"`. The backend bakes a trailing `(WxH, Primary)`, `(Primary)` or `(WxH)` onto display labels - see `parseTarget`, which splits that back out for display.
- `kind?: string` - `"display"` or `"window"`. *Why optional:* older cached/mocked data may omit it; callers that group by kind treat a missing value as `"display"` (i.e. `kind !== "window"`).

### Used by

- `src/hud/hooks/useDevices.ts` - stores the list returned by `listDisplays`
- `src/hud/devices/TargetSheet.tsx` - groups the list into displays (drawn as a map) vs. windows (a list)
- `src/hud/components/IdleCard.tsx` - shows the chosen target on the screen row
- `src/hud/Hud.tsx` - passes the list to `IdleCard`

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

## resolveSelection

```ts
export function resolveSelection(prev: DeviceState, displays: DisplayInfo[], mics: AudioInfo[]): DeviceState
```

Re-resolves a device selection against a fresh enumeration: keeps each of `prev.displayId`/`prev.micId` if it is still present in the corresponding new list, otherwise falls back to that list's first entry (or `null`).

### Inputs

- `prev: DeviceState` - the selection before this enumeration. Passing `{ displayId: null, micId: null }` (the initial state) makes this behave identically to `pickDefaults`.
- `displays: DisplayInfo[]`, `mics: AudioInfo[]` - a fresh `listDisplays`/`listAudioInputs` result.

### Returns

`DeviceState` with each field kept if still valid, else defaulted to that list's first id (or `null` if the list is empty).

### Behaviors (pinned by unit tests in `src/hud/devices/selectDevices.test.ts`)

- Keeps the current pick when it is still present in the fresh lists.
- Falls back to the first mic when the picked one has disappeared (unplugged).
- Falls back to `null` when the picked device is gone and none remain.
- An initial `{ displayId: null, micId: null }` selection resolves the same way `pickDefaults` does.

### Used by

- `src/hud/hooks/useDevices.ts` - called on mount and on every `devicechange` event, so a plug/unplug can't leave `sel` pointing at a device that no longer exists.

## parseTarget

```ts
export function parseTarget(t: DisplayInfo, index: number): { title: string; resolution: string | null; primary: boolean }
```

Splits a `DisplayInfo.label` into a clean `title` plus the pieces the card's screen row and `TargetSheet` render as a resolution sub-line, a "Primary" badge and a to-scale rectangle, instead of showing the backend's raw parenthetical text verbatim. For a window target, delegates the whole `title` to `prettifyWindowLabel` instead (windows never carry a resolution/Primary suffix).

### Inputs

- `t: DisplayInfo` - one target from a `listDisplays()` result.
- `index: number` - `t`'s position in the raw (unfiltered) `listDisplays()` array. *Why the caller must pass it:* the backend enumerates the main monitor first, before any other display or window, so raw index 0 is primary even when a label carries no `Primary` word (a display whose size it could not read) - a target's position within a post-filtering subgroup would give the wrong answer once windows are involved. A label that says `Primary` is primary regardless of index.

### Returns

- `title: string` - for `t.kind === "window"`, `prettifyWindowLabel(t.label)`. Otherwise `t.label` with a trailing `" (WxH, Primary)"`, `" (Primary)"` or `" (WxH)"` stripped, or `t.label` unchanged if it matches none of those (a display whose resolution the backend couldn't read).
- `resolution: string | null` - `null` for a window; otherwise the extracted `"WxH"` substring, or `null` when the label had no resolution suffix (the primary display, or one the backend can't measure).
- `primary: boolean` - `index === 0 && t.kind !== "window"`. Always `false` for windows, even a hypothetical one at index 0.

### Used by

- `src/hud/devices/TargetSheet.tsx` - calls it per row to render the title/resolution/badge

## cleanDeviceLabel

```ts
export function cleanDeviceLabel(raw: string): string
```

Strips Windows/cpal's device-name packaging for display in a dropdown - only the label changes; the raw string a caller already holds (`sel.micId`, `camId`) still round-trips into `startRecording`/`getUserMedia` unchanged.

Two independent shapes are unwrapped, in order:

1. A generic Windows audio category wrapping the real product name in parens, e.g. `"Microphone (3- Insta360 Link 2C)"`, `"Headset (WH-1000XM4 Hands-Free AG Audio)"`, `"Microphone Array (Realtek High Definition Audio)"` (a laptop's built-in array mic), or `"Headset Microphone (2- Realtek(R) Audio)"` (a Bluetooth/USB headset's mic endpoint). Recognized categories: `Microphone` (optionally `+ " Array"`), `Headset` (optionally `+ " Microphone"` or `+ " Earphone"`), `Headphones`, `Speaker(s)`, `Line In` - both the one-word and two-word forms Windows actually reports. The match is greedy to the *last* `)`, so a nested-paren product name like `"Microphone (Realtek(R) Audio)"` survives intact as `"Realtek(R) Audio"`.
2. cpal/Windows' own numeric enumeration prefix inside that name, e.g. `"3- Insta360 Link 2C"` -> `"Insta360 Link 2C"`.

A trailing `" (Windows Virtual Camera)"` suffix (some webcams report this via their MediaFoundation frame-server shim) is also dropped. *Decision, documented not inferred:* this names a driver implementation detail, not the physical device, and the same hardware's cpal *mic* label never carries an analogous suffix - keeping it would make one physical device read as two unrelated ones across the mic and camera dropdowns.

### Inputs

- `raw: string` - a `DisplayInfo.label`/`AudioInfo.label`/browser `MediaDeviceInfo.label` as reported by cpal, `list_audio_inputs`, or `navigator.mediaDevices.enumerateDevices()`.

### Returns

The cleaned label, or `raw.trim()` unchanged if none of the patterns above matched.

### Behaviors (pinned by unit tests in `src/hud/devices/selectDevices.test.ts`)

- Unwraps a category-wrapped name with a numeric prefix: `"Microphone (3- Insta360 Link 2C)"` -> `"Insta360 Link 2C"`.
- Preserves nested parens in the unwrapped name.
- Strips a numeric prefix with no category wrapper.
- Drops the Windows Virtual Camera suffix.
- Leaves an already-clean label untouched.
- Unwraps the two-word `"Headset Microphone"` and `"Headset Earphone"` categories.
- Unwraps the two-word `"Microphone Array"` category.
- Leaves a two-word category name with no parenthetical (nothing to unwrap) untouched.

### Used by

- `src/hud/Hud.tsx` - applied to every mic (`Dropdown`) and camera (`Dropdown`) option label.

## isOwnProcessWindow

```ts
export function isOwnProcessWindow(t: DisplayInfo): boolean
```

`list_displays` (`commands.rs`) filters the exact string `"TCursor"` out of the window list - the HUD's own titled window. It does not catch a second, distinct case: before a window's title is set (or for a window Windows never gave one), `EnumWindows` can report the raw executable path as the title instead, e.g. `"App: C:\Users\...\tcursor-scaffold.exe"` for this very process. Since the Rust side is unchanged here, this is a frontend-only second line of defense so the app never lists itself as a capture target.

### Inputs

- `t: DisplayInfo` - one target from a `listDisplays()` result.

### Returns

`true` when `t.kind === "window"` and `t.label` ends with `tcursor-scaffold.exe` (case-insensitive, with or without a preceding path separator).

### Used by

- `src/hud/devices/TargetSheet.tsx` - filters this process's window out of the rendered rows.

## prettifyWindowLabel

```ts
export function prettifyWindowLabel(label: string): string
```

A window title that is itself a filesystem path (the same raw-path shape `isOwnProcessWindow` looks for, but for *any* process, not just this one) reads far better as its filename with no extension than as the full path.

### Inputs

- `label: string` - a window `DisplayInfo.label`, i.e. `"App: <raw title>"`.

### Returns

If the text after `"App: "` looks like a Windows path (`C:\...` or `\\...`), `"App: "` plus that path's last segment with any short (1-6 char) trailing extension removed - e.g. `"App: C:\Users\...\tcursor-scaffold.exe"` -> `"App: tcursor-scaffold"`. Otherwise `label` unchanged.

### Used by

- `src/hud/devices/selectDevices.ts` - `parseTarget` delegates a window's `title` to this function.
