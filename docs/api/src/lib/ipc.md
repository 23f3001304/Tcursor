# src/lib/ipc.ts

Typed wrappers over Tauri's `invoke` for every backend command the frontend calls. Each export is a one-liner that maps a typed TypeScript call to the matching Rust `#[tauri::command]`. No logic lives here; this file is the boundary between the frontend and the Tauri process.

## listDisplays

```ts
export const listDisplays = () => invoke<DisplayInfo[]>("list_displays")
```

### Returns

`Promise<DisplayInfo[]>` - array of connected displays. Each entry has the id and label fields defined in `DisplayInfo` (from `../hud/selectDevices`).

### Used by

`useDevices` (`src/hud/hooks/useDevices.ts`) - calls on mount to populate the screen capture dropdown.

## listAudioInputs

```ts
export const listAudioInputs = () => invoke<AudioInfo[]>("list_audio_inputs")
```

### Returns

`Promise<AudioInfo[]>` - array of available audio input devices.

### Used by

`useDevices` (`src/hud/hooks/useDevices.ts`) - calls on mount to populate the microphone dropdown.

## startRecording

```ts
export const startRecording = (projectName: string, micId: string | null, systemAudio: boolean, gameMode: boolean) => invoke<void>("start_recording", { projectName, micId, systemAudio, gameMode })
```

### Inputs

- `projectName` (`string`) - unique folder name for this session (e.g., `"rec-<timestamp>"`). *Why caller-provided:* the frontend generates the name so the folder path is known before the recording starts, allowing webcam bytes to be saved to the same folder immediately after stopping.
- `micId` (`string | null`) - OS audio device id, or `null` to skip microphone capture. *Why nullable:* the user may have disabled the mic toggle.
- `systemAudio` (`boolean`) - whether to capture system (loopback) audio.
- `gameMode` (`boolean`) - when true, switches the video encoder to CFR pacing for smooth game / variable-FPS content.

### Returns

`Promise<void>`. Rejects with a step-tagged error string (e.g., `"prepare folder: ..."`) so the caller can display the specific failure stage.

### Used by

`Hud` (`src/hud/Hud.tsx`) - called at the start of `toggle()`.

## pauseRecording

```ts
export const pauseRecording = () => invoke<void>("pause_recording")
```

### Returns

`Promise<void>`.

### Used by

`Hud` (`src/hud/Hud.tsx`) - called from `togglePause()`.

## resumeRecording

```ts
export const resumeRecording = () => invoke<void>("resume_recording")
```

### Returns

`Promise<void>`.

### Used by

`Hud` (`src/hud/Hud.tsx`) - called from `togglePause()`.

## stopRecording

```ts
export const stopRecording = () => invoke<{ folder: string; frames: number }>("stop_recording")
```

### Returns

`Promise<{ folder: string; frames: number }>`. `folder` is the absolute path to the project directory written by the Rust side. `frames` is the total number of captured frames (used for diagnostics). *Why return the folder:* the frontend does not know the absolute path ahead of time on all platforms; the backend resolves it.

### Used by

`Hud` (`src/hud/Hud.tsx`) - called at the start of the stop path in `toggle()`. The returned `folder` is passed to `saveWebcam` and `exportProject`.

## saveWebcam

```ts
export const saveWebcam = (folder: string, bytes: number[]) => invoke<void>("save_webcam", { folder, bytes })
```

### Inputs

- `folder` (`string`) - absolute path to the project directory. *Why needed:* the backend writes the webcam file alongside the screen capture.
- `bytes` (`number[]`) - raw webcam recording bytes as a plain number array. *Why `number[]` rather than `Uint8Array`:* Tauri's IPC bridge cannot serialize typed arrays directly; the caller converts before passing.

### Returns

`Promise<void>`.

### Used by

`Hud` (`src/hud/Hud.tsx`) - called in the stop path when `webcam.stop()` returns bytes.

## exportProject

```ts
export const exportProject = (folder: string) => invoke<void>("export_project", { folder })
```

### Inputs

- `folder` (`string`) - absolute path to the project directory to render.

### Returns

`Promise<void>` that resolves when the export pipeline has been *started*, not when it finishes. Progress and completion arrive asynchronously via three Tauri events: `export-progress` (payload `number`), `export-done` (payload `string` folder path), and `export-error`.

### Used by

`Hud` (`src/hud/Hud.tsx`) - called at the end of the stop path in `toggle()`.

## getSettings

```ts
export const getSettings = () => invoke<Settings>("get_settings")
```

### Returns

`Promise<Settings>` - the full persisted settings object as typed in `../hud/settings`.

### Used by

- `Hud` (`src/hud/Hud.tsx`) - reads on mount to apply the initial theme.
- `Settings` (`src/hud/settings/SettingsPanel.tsx`) - reads on mount to seed the draft.
- `Preferences` (`src/hud/preferences/Preferences.tsx`) - reads on mount to seed the draft.

## setSettings

```ts
export const setSettings = (settings: Settings) => invoke<void>("set_settings", { settings })
```

### Inputs

- `settings` (`Settings`) - the full settings object to persist. *Why full object:* the Rust handler atomically replaces the file; partial updates would require a merge on the Rust side.

### Returns

`Promise<void>`.

### Used by

- `Settings` (`src/hud/settings/SettingsPanel.tsx`) - called from `patch()` on every user interaction.
- `Preferences` (`src/hud/preferences/Preferences.tsx`) - called from `patch()` on every user interaction.

## getEdit

```ts
export const getEdit = (folder: string) => invoke<EditDoc>("get_edit", { folder })
```

### Inputs

- `folder` (`string`) - absolute path to the project directory. *Why not a project id:* the folder is the stable identity for a recording session.

### Returns

`Promise<EditDoc>` - the current edit document for that session, seeded from the recording's auto-zoom output on first call (`load_or_seed` on the Rust side). The returned doc is byte-identical to the legacy export path when untouched. *Why seeded on first call rather than at stop time:* lazy seeding keeps the stop path fast; seeding is deferred until the editor is actually opened.

### Implementation

Not yet consumed by any `.tsx` file (the M3 editor UI is in progress); the function is defined here so the backend command surface is available when the editor is wired up.

## applyEditOp

```ts
export const applyEditOp = (folder: string, op: EditOp) => invoke<EditDoc>("apply_edit_op", { folder, op })
```

### Inputs

- `folder` (`string`) - project directory.
- `op` (`EditOp`) - the discriminated-union edit verb to apply (see `src/lib/edit.ts`). *Why one op at a time:* each call atomically applies, persists `edit.json`, and returns the updated doc; the editor never holds stale state.

### Returns

`Promise<EditDoc>` - the updated document after the op is applied. This is the editor's primary mutation entry point.

### Implementation

Not yet consumed by any `.tsx` file; defined in anticipation of the M3 editor.

## saveEdit

```ts
export const saveEdit = (folder: string, doc: EditDoc) => invoke<void>("save_edit", { folder, doc })
```

### Inputs

- `folder` (`string`) - project directory.
- `doc` (`EditDoc`) - full document to persist. *Why a bulk save alongside `applyEditOp`:* used for GUI saves or batch imports where emitting individual ops would be expensive.

### Returns

`Promise<void>`.

## aiAutoedit

```ts
export const aiAutoedit = (folder: string, model?: string) => invoke<EditDoc>("ai_autoedit", { folder, model })
```

### Inputs

- `folder` (`string`) - project directory.
- `model` (`string`, optional) - Ollama model name override. *Why optional:* the backend has a compiled-in default; the frontend passes a value only when the user has chosen a different model in settings.

### Returns

`Promise<EditDoc>` - the rewritten document after the AI director has applied zooms, trim, and speed segments. Rejects (and saves nothing) if Ollama is unreachable or the reply is unparseable. *Why return the doc rather than void:* the editor must refresh its state immediately after the AI pass without a separate `getEdit` round-trip.

## setCapturable

```ts
export const setCapturable = (capturable: boolean) => invoke<boolean>("set_capturable", { capturable })
```

### Inputs

- `capturable` (`boolean`) - true to make the app window visible to screen capture (the editor), false to hide it (the HUD). *Why:* the HUD is capture-excluded so it never appears in the user's recordings; the editor opts back in so it can be screenshotted/recorded.

### Returns

`Promise<boolean>` - true if the OS display-affinity call succeeded.

### Used by

`App` (`src/App.tsx`) - called on view switch: `true` when opening the editor, `false` when returning to the HUD.

## CamSample

```ts
export interface CamSample { t: number; scale: number; cx: number; cy: number; curx: number; cury: number }
```

One sample of the camera curve (mirrors the Rust `CamSample`): output time `t` in ms, the zoom as `scale` + centre `cx`/`cy`, and the cursor `curx`/`cury` - all 0..1 fractions of the screen content. The editor interpolates these per animation frame to drive the smooth composite preview.

## cameraTrack

```ts
export const cameraTrack = (folder: string) => invoke<CamSample[]>("camera_track", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<CamSample[]>` - the exact per-frame camera curve for the whole timeline (instant: pure math, cached on the backend). The editor refetches it whenever an edit changes the timeline, then plays the recording natively and applies this curve on the canvas.

## PreviewLayout

```ts
export interface PreviewLayout { screen: [number, number, number, number]; radius: number; cam: [number, number, number, number, number] | null }
```

The static export framing as fractions of the output (mirrors the Rust `PreviewLayout`): `screen` is the screen rect `[x, y, w, h]`, `radius` the corner radius (fraction of width), and `cam` the webcam PiP rect `[x, y, w, h, radius]` or `null` when hidden. The canvas compositor frames the screen and webcam from this so the preview matches the export.

## previewLayout

```ts
export const previewLayout = (folder: string) => invoke<PreviewLayout>("preview_layout", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<PreviewLayout>` - the screen/webcam framing fractions for the recording.

## ensureProxy

```ts
export const ensureProxy = (folder: string, height: number) => invoke<string>("ensure_proxy", { folder, height })
```

### Inputs

- `folder` (`string`) - project directory.
- `height` (`number`) - target proxy height in px (e.g. 480/720/1080). *Why:* the editor plays a light proxy sized to the display rather than decoding the raw 4K capture.

### Returns

`Promise<string>` - the proxy file path (transcoded once, then cached). The frontend wraps it with `fileSrc` to get an asset-protocol URL for a `<video>`.

## fileSrc

```ts
export const fileSrc = (path: string) => convertFileSrc(path)
```

### Inputs

- `path` (`string`) - absolute local file path.

### Returns

`string` - an asset-protocol URL usable as a `<video>`/`<img>` `src`. *Why:* native media elements cannot load raw `file://` paths under Tauri's security model; `convertFileSrc` maps the path to the allowed asset scope.

## ClickSample

```ts
export interface ClickSample { t: number; x: number; y: number }
```

One click ripple (mirrors the Rust `ClickSample`): output time `t` in ms and `x`/`y` as 0..1 fractions of the screen content - the same basis as `CamSample`'s cursor. The canvas compositor draws an expanding ring at each click within ~500ms of the playhead, mapped through the current zoom.

## clickTrack

```ts
export const clickTrack = (folder: string) => invoke<ClickSample[]>("click_track", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<ClickSample[]>` - the mouse-down track for the whole timeline, so the editor can draw click ripples matching the export's click FX.

## previewBg

```ts
export const previewBg = (folder: string) => invoke<string>("preview_bg", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<string>` - a `data:image/png;base64,...` URL of the export background (mesh/gradient), so the canvas preview paints the exact same background the export uses. The frontend decodes it into an `<img>` the compositor draws under the screen.

## CursorSpriteDto

```ts
export interface CursorSpriteDto { kind: string; url: string; hot: [number, number]; canvas_h: number }
```

One cursor sprite (mirrors the Rust `CursorSpriteDto`): the lowercase type name, a PNG data URL (cropped + dark-inverted like the export), the hotspot (0..1 of the cropped sprite), and the original canvas height for uniform scaling. The preview decodes each into an `<img>` keyed by `kind`.

## cursorSprites

```ts
export const cursorSprites = (folder: string) => invoke<CursorSpriteDto[]>("cursor_sprites", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<CursorSpriteDto[]>` - the Capitaine pack so the preview can draw the real cursor (Enhanced style) instead of an arrow.

## CursorKindSample

```ts
export interface CursorKindSample { t: number; kind: string }
```

One cursor-shape change at output time `t` (ms); `kind` is the lowercase cursor-type name. The preview binary-searches this track for the active sprite.

## cursorKinds

```ts
export const cursorKinds = (folder: string) => invoke<CursorKindSample[]>("cursor_kinds", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<CursorKindSample[]>` - the cursor-type track in output time.

## ensureThumbs

```ts
export const ensureThumbs = (folder: string, count: number) => invoke<string[]>("ensure_thumbs", { folder, count })
```

### Inputs

- `folder` (`string`) - project directory.
- `count` (`number`) - number of filmstrip thumbnails (the backend clamps 8..120).

### Returns

`Promise<string[]>` - thumbnail file paths (wrap each with `fileSrc`); one cached ffmpeg pass.

## ensureWaveform

```ts
export const ensureWaveform = (folder: string, which: "system" | "mic") => invoke<string>("ensure_waveform", { folder, which })
```

### Inputs

- `folder` (`string`) - project directory.
- `which` (`"system" | "mic"`) - which source's waveform to render.

### Returns

`Promise<string>` - a cached waveform PNG path, or `""` when that source wasn't recorded.

## ensurePreviewAudio

```ts
export const ensurePreviewAudio = (folder: string) => invoke<string>("ensure_preview_audio", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<string>` - a cached mixed (mic+system) preview-audio file path so the editor can play sound, or `""` when neither source exists.
