# src/lib/ipc.ts

Typed wrappers over Tauri's `invoke` for every backend command the frontend calls. Each export is a one-liner that maps a typed TypeScript call to the matching Rust `#[tauri::command]`. No logic lives here; this file is the boundary between the frontend and the Tauri process.

The canvas-preview compositing IPC (camera curve, layout panel rects, click track, FX overlay - `CamSample` through `previewFxOverlay`) lives in `ipcPreview.ts` instead, split out purely for file size; `ipc.ts` re-exports it (`export * from "./ipcPreview"`) so every `from "../../lib/ipc"` import is unaffected. See `ipcPreview.md`.

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
export const startRecording = (projectName: string, micId: string | null, targetId: string | null, systemAudio: boolean, gameMode: boolean) => invoke<string>("start_recording", { projectName, micId, targetId, systemAudio, gameMode })
```

### Inputs

- `projectName` (`string`) - unique folder name for this session (e.g., `"rec-<timestamp>"`). *Why caller-provided:* the frontend generates the name so it can pass the same name straight into `webcam.start()`, before the backend has resolved anything.
- `micId` (`string | null`) - OS audio device id, or `null` to skip microphone capture. *Why nullable:* the user may have disabled the mic toggle.
- `targetId` (`string | null`) - id of the display (or window) to capture, or `null` for the default target. *Why nullable:* mirrors `DeviceState.displayId`, which starts out `null` until `listDisplays` resolves.
- `systemAudio` (`boolean`) - whether to capture system (loopback) audio.
- `gameMode` (`boolean`) - when true, switches the video encoder to CFR pacing for smooth game / variable-FPS content.

### Returns

`Promise<string>` - the resolved absolute project folder path. Rejects with a step-tagged error string (e.g., `"prepare folder: ..."`) so the caller can display the specific failure stage.

### Used by

`useRecordingFlow` (`src/hud/hooks/useRecordingFlow.ts`) - called at the start of `toggle()`; the resolved folder is passed to `webcam.start()` and threaded through to `stopRecording`/`preprocessProject`/`onEdit`.

## pauseRecording

```ts
export const pauseRecording = () => invoke<void>("pause_recording")
```

### Returns

`Promise<void>`.

### Used by

`useRecordingFlow` (`src/hud/hooks/useRecordingFlow.ts`) - called from `togglePause()`.

## resumeRecording

```ts
export const resumeRecording = () => invoke<void>("resume_recording")
```

### Returns

`Promise<void>`.

### Used by

`useRecordingFlow` (`src/hud/hooks/useRecordingFlow.ts`) - called from `togglePause()`.

## stopRecording

```ts
export const stopRecording = () => invoke<{ folder: string; frames: number }>("stop_recording")
```

### Returns

`Promise<{ folder: string; frames: number }>`. `folder` is the absolute path to the project directory written by the Rust side. `frames` is the total number of captured frames (used for diagnostics). *Why return the folder:* the frontend does not know the absolute path ahead of time on all platforms; the backend resolves it.

### Used by

`useRecordingFlow` (`src/hud/hooks/useRecordingFlow.ts`) - called at the start of the stop path in `toggle()`. The returned `folder` is passed to `preprocessProject`, then (once that finishes) to `onEdit`.

## saveWebcam

```ts
export const saveWebcam = (folder: string, bytes: Uint8Array) => invoke<void>("save_webcam", { folder, bytes })
```

### Inputs

- `folder` (`string`) - absolute path to the project directory. *Why needed:* the backend writes the webcam file alongside the screen capture.
- `bytes` (`Uint8Array`) - the whole webcam recording as one blob.

### Returns

`Promise<void>`.

### Used by

Not currently called from any `.tsx` file - superseded by `appendWebcam`, which streams the recording to disk in chunks *during* capture instead of holding the whole clip in memory for one write at Stop. `save_webcam` still exists on the Rust side as a simpler one-shot entry point, just unused today.

## appendWebcam

```ts
export const appendWebcam = (folder: string, bytes: Uint8Array) => invoke<void>("append_webcam", { folder, bytes })
```

### Inputs

- `folder` (`string`) - absolute path to the project directory.
- `bytes` (`Uint8Array`) - one `MediaRecorder` chunk (a 1s timeslice) to append to `webcam.webm`.

### Returns

`Promise<void>`.

### Used by

`useWebcamRecorder` (`src/hud/hooks/useWebcamRecorder.ts`) - called from the recorder's `ondataavailable` handler for every chunk, chained so appends land in order; `stop()` awaits the chain so `webcam.webm` is complete before the editor opens.

## ExportResolution

```ts
export type ExportResolution = "p720" | "p1080" | "p1440" | "p2160" | "source"
```

Output frame SIZE (mirrors Rust `export::settings::Resolution`), independent of the doc's `Aspect` (the RATIO). Fixed presets name the SHORT edge in px - `"p1080"` on a landscape aspect is height=1080 (1920x1080); on a portrait aspect the short edge is the WIDTH (1080x1920). `"source"` (the default) keeps whatever the aspect alone resolves to.

## ExportFps

```ts
export type ExportFps = "f30" | "f60" | "source"
```

Output frame rate (mirrors Rust `export::settings::Fps`). `"source"` matches the capture/display refresh rate (capped at 60); `"f60"` (the default) is a fixed 60 regardless of it.

## ExportFormat

```ts
export type ExportFormat = "mp4" | "webm" | "gif"
```

Export container/codec (mirrors Rust `export::settings::Format`). `"gif"` cannot carry audio.

## ExportSettings

```ts
export interface ExportSettings { resolution: ExportResolution; fps: ExportFps; quality_crf: number; format: ExportFormat }
```

User-chosen export settings, collected by `ExportDialog` and sent to `export_project` - mirrors Rust `export::settings::ExportSettings`.

### Used by

- `ExportDialog` (`src/editor/shell/ExportDialog.tsx`) - owns the local draft the user edits, initialized from `DEFAULT_EXPORT_SETTINGS`.

## DEFAULT_EXPORT_SETTINGS

```ts
export const DEFAULT_EXPORT_SETTINGS: ExportSettings = { resolution: "source", fps: "f60", quality_crf: 24, format: "mp4" }
```

`ExportSettings::default()` on the Rust side: this is the exact today's-export configuration (60fps, CRF 24, MP4/H.264) - opening `ExportDialog` for the first time reproduces the pre-`ExportSettings` export if the user clicks Export without changing anything.

## exportProject

```ts
export const exportProject = (folder: string, settings: ExportSettings) => invoke<void>("export_project", { folder, settings })
```

### Inputs

- `folder` (`string`) - absolute path to the project directory to render.
- `settings` (`ExportSettings`) - the resolution/fps/quality/format the user chose in `ExportDialog`.

### Returns

`Promise<void>` that resolves when the export pipeline has been *started*, not when it finishes. Progress and completion arrive asynchronously via three Tauri events: `export-progress` (payload `number`), `export-done` (payload `string` folder path), and `export-error` (payload `string` error message).

### Used by

`Editor` (`src/editor/Editor.tsx`) - called from the `onExport` callback passed to `ExportDialog`, after resetting the lifted `exporting`/`pct`/`exportDone`/`exportError` state from `useEditorData`.

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

### Used by

`useEditorData` (`src/editor/hooks/useEditorData.ts`) - fetched once per `[folder]` into `doc`.

## applyEditOp

```ts
export const applyEditOp = (folder: string, op: EditOp) => invoke<EditDoc>("apply_edit_op", { folder, op })
```

### Inputs

- `folder` (`string`) - project directory.
- `op` (`EditOp`) - the discriminated-union edit verb to apply (see `src/lib/edit.ts`). *Why one op at a time:* each call atomically applies, persists `edit.json`, and returns the updated doc; the editor never holds stale state.

### Returns

`Promise<EditDoc>` - the updated document after the op is applied. This is the editor's primary mutation entry point.

### Used by

`Editor` (`src/editor/Editor.tsx`) - wrapped by the local `applyOp`, which records undo history and bumps `rev` around every call; the wrapped version is threaded as a prop into every inspector and panel, making this the editor's primary mutation entry point.

## saveEdit

```ts
export const saveEdit = (folder: string, doc: EditDoc) => invoke<void>("save_edit", { folder, doc })
```

### Inputs

- `folder` (`string`) - project directory.
- `doc` (`EditDoc`) - full document to persist. *Why a bulk save alongside `applyEditOp`:* used for GUI saves or batch imports where emitting individual ops would be expensive.

### Returns

`Promise<void>`.

### Used by

- `useEditHistory` (`src/editor/hooks/useEditHistory.ts`) - `undo`/`redo` persist the restored doc via a bulk `saveEdit` rather than replaying it as an `EditOp`.
- `Editor` (`src/editor/Editor.tsx`) - `saveDocSettings` persists the whole doc after patching `settings` (theme/cursor/background/audio/etc. aren't `EditOp`s).

## AiStep

```ts
export type AiStep = { op: EditOp; label: string };
```

One labeled step of the AI director's plan: the `EditOp` to apply plus a human "what I did + why" line - mirrors the Rust `AiStep` (`src-tauri/src/ai/commands.rs`).

### Used by

- `src/lib/ipc.ts` - element type of `aiPlan`'s returned array.
- `src/editor/Editor.tsx` - `onRun` applies each step's `op` via `applyEditOp` and appends its `label` to the on-screen agentic log.

## aiPlan

```ts
export const aiPlan = (folder: string, model?: string) => invoke<AiStep[]>("ai_plan", { folder, model })
```

The AI director's plan as ordered, labeled steps (NOT applied) - the editor reveals them one-by-one via `applyEditOp` for the agentic feel.

### Inputs

- `folder` (`string`) - project directory.
- `model` (`string`, optional) - Ollama model name override. *Why optional:* falls back to the first installed Ollama chat model when this is omitted, empty, or names a model that isn't installed - there is no hardcoded default model name (a missing model used to 404, which was the "AI returned 404" bug).

### Returns

`Promise<AiStep[]>` - the plan as ordered, labeled steps, not yet applied to the doc. When the doc already has zooms, the first step is a `clear_zooms` op labeled "Rethinking your zooms…", so the reveal shows the mechanical seed-time zooms give way to the smart ones. Rejects if Ollama is unreachable or the reply is unparseable.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `onRun` fetches the whole plan in one call, then reveals it by applying each step's `op` via `applyEditOp` and narrating its `label` roughly 460ms apart, so auto-edit reads like a live agent editing the panels rather than an instant bulk change. One `record(doc)` before the loop makes the whole reveal a single undo step.

## listOllamaModels

```ts
export const listOllamaModels = () => invoke<string[]>("list_ollama_models")
```

Locally-installed Ollama model names, for the AI panel's Engine picker.

### Returns

`Promise<string[]>` - chat model names only (embedding-only models are filtered out on the Rust side). Never rejects: resolves to `[]` when Ollama isn't running, since this is a convenience for populating a dropdown, not a precondition for `aiPlan`.

### Used by

`AiPanel` (`src/editor/panels/AiPanel.tsx`) - fetched once on mount to populate the Engine picker; falls back to showing just the currently-selected (or default) model name when the list is empty.

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

## ensureProxy

```ts
export const ensureProxy = (folder: string, height: number) => invoke<string>("ensure_proxy", { folder, height })
```

### Inputs

- `folder` (`string`) - project directory.
- `height` (`number`) - target proxy height in px (e.g. 480/720/1080). *Why:* the editor plays a light proxy sized to the display rather than decoding the raw 4K capture.

### Returns

`Promise<string>` - the proxy file path (transcoded once, then cached). The frontend wraps it with `fileSrc` to get an asset-protocol URL for a `<video>`.

## DEFAULT_PROXY_HEIGHT

```ts
export const DEFAULT_PROXY_HEIGHT = 720
```

Proxy height `preprocess_project` transcodes ahead of time (mirrors the Rust `preprocess::DEFAULT_PROXY_HEIGHT`). Also `Editor.tsx`'s initial `quality` state, so a freshly preprocessed project's default quality always matches what preprocessing already put on disk - the one place this number is spelled, so the two sides can never drift apart.

### Used by

- `Editor` (`src/editor/Editor.tsx`) - initial `quality` state.
- `useEditorData` (`src/editor/hooks/useEditorData.ts`) - the proxy-skip condition (`preprocessed && quality === DEFAULT_PROXY_HEIGHT`).

## fileSrc

```ts
export const fileSrc = (path: string) => convertFileSrc(path)
```

### Inputs

- `path` (`string`) - absolute local file path.

### Returns

`string` - an asset-protocol URL usable as a `<video>`/`<img>` `src`. *Why:* native media elements cannot load raw `file://` paths under Tauri's security model; `convertFileSrc` maps the path to the allowed asset scope.

## CursorSpriteDto

```ts
export interface CursorSpriteDto { kind: string; url: string; hot: [number, number]; canvas_h: number }
```

One cursor sprite (mirrors the Rust `CursorSpriteDto`): the lowercase type name, a PNG data URL (cropped + dark-inverted like the export), the hotspot (0..1 of the cropped sprite), and the original canvas height for uniform scaling. The preview decodes each into an `<img>` keyed by `kind`.

## BusySpecDto

```ts
export interface BusySpecDto { anim: "spin" | "flip" | "pulse"; fps: number; frames: number }
```

How a pack animates its busy cursor (pack format v2), mirroring Rust `BusySpec`. `frames` is how many explicit `busy_NN.png` files the pack ships - `0` means `anim` synthesises the animation from the single `busy.png` instead. Fed to `busyPose` (`src/editor/stage/cursorBusy.ts`), the TS mirror of the export's own `busy_pose`.

## CursorPackDto

```ts
export interface CursorPackDto {
  sprites: CursorSpriteDto[]; busy_frames: CursorSpriteDto[]; busy: BusySpecDto | null;
}
```

The recording's selected cursor pack, ready to draw: one sprite per kind, the pack's explicit busy frames (empty unless it ships them), and its declared busy animation (`null` for the embedded set and any v1 pack). `busy` + `busy_frames` are what let the preview run the same `busyPose` the export does.

## cursorSprites

```ts
export const cursorSprites = (folder: string) => invoke<CursorPackDto>("cursor_sprites", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<CursorPackDto>` - the recording's selected pack (embedded, bundled, or imported), decoded to match the export exactly, so the preview draws the real cursor rather than an arrow.

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

## CapturedCursorDto

```ts
export interface CapturedCursorDto { id: number; w: number; h: number; hx: number; hy: number; url: string }
```

One captured OS cursor bitmap (mirrors the Rust `CapturedCursorDto`): its layer id, pixel size, hotspot in PIXELS from the top-left, and the recorded PNG as a data URL. This is the real cursor that was on screen, not a sprite-pack stand-in, so the PNG is passed through untouched - no crop, no dark-invert.

## CursorLayerDto

```ts
export interface CursorLayerDto {
  cursors: CapturedCursorDto[]; track: [number, number][]; src_w: number; src_h: number;
}
```

The recording's captured OS-cursor layer: the bitmaps, `[t, id]` samples in output ms saying which was showing, and the recorded video's own pixel size. The bitmaps are in SOURCE pixels, so `src_w` is what `contentScale` uses to size them relative to the screen content - the preview's `<video>` is a downscaled proxy and cannot supply it. `src_w` is `0` only when the backend could not probe the video. `null` (not an empty object) for a recording made before the layer existed.

## cursorLayer

```ts
export const cursorLayer = (folder: string) => invoke<CursorLayerDto | null>("cursor_layer", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<CursorLayerDto | null>` - the layer, or `null` for a pre-layer recording. `Stage` passes it down only when the doc's cursor style is `"system"`; `useCursorSprites` decodes it into a `CapturedLayer`, and `drawCursorSprite` composites the real cursor from it instead of the synthetic arrow.

## CursorPackInfo

```ts
export interface CursorPackInfo {
  id: string; name: string; builtin: boolean; dir: string;
  files: Record<string, string>; busy: BusySpecDto | null;
}
```

One selectable cursor pack. `id` persists into `CursorSettings.pack`; `builtin` marks a pack the user cannot delete (the embedded set, always first, or one bundled with the app).

`dir` is the pack's folder, so the grid loads each tile's sprite through the asset protocol rather than the backend base64ing every pack's nine PNGs into one reply. The embedded set has one too now (`assets/cursors`, shipped as a resource) - which is what lets its tile show a sprite - even though the EXPORT still reads the copies compiled into the binary.

`files` maps each kind wire name to its filename inside `dir`, already alias-resolved (the embedded pack spells its arrow `pointer.png`) and already carrying the busy-is-arrow substitution, so the grid has to know neither rule; a kind the pack does not ship is absent. `busy` is the pack's busy animation, so a hovered tile previews it with the same `busyPose` the export runs - `null` for the embedded set, whose busy state renders as the arrow.

## listCursorPacks

```ts
export const listCursorPacks = () => invoke<CursorPackInfo[]>("list_cursor_packs")
```

### Returns

`Promise<CursorPackInfo[]>` - the built-in pack first, then every pack previously imported via `importCursorPack`.

### Used by

`CursorPanel` (`src/editor/panels/CursorPanel.tsx`) - fetched on mount to populate the pack picker.

## importCursorPack

```ts
export const importCursorPack = (path: string) => invoke<CursorPackInfo>("import_cursor_pack", { path })
```

### Inputs

- `path` (`string`) - absolute path to a folder the user picked via the Tauri dialog plugin's folder picker. *Why a folder path rather than file contents:* the Rust side reads and validates the PNGs directly off disk, so only the path needs to cross the IPC boundary.

### Returns

`Promise<CursorPackInfo>` - the newly imported pack (`builtin: false`). Rejects with a message when the folder has no recognized cursor PNGs (`arrow.png`, `hand.png`, `ibeam.png`, ...) or an invalid `hotspots.json`.

### Used by

`CursorPanel` (`src/editor/panels/CursorPanel.tsx`) - called from the "Import pack..." button; on success, appends the result to the local pack list and selects it.

## BackgroundAssetInfo

```ts
export interface BackgroundAssetInfo { rel_path: string; kind: "image" | "video"; width: number; height: number; duration_ms: number | null }
```

An imported background file - mirrors Rust `settings::bg_asset::BackgroundAssetInfo`. `rel_path` is always relative to the project folder and forward-slashed (`background/<file>`): that is what `settings.background.asset` stores, and the reason a project folder stays portable. `duration_ms` is `null` for a still; a video's is what the preview loops on.

## importBackgroundAsset

```ts
export const importBackgroundAsset = (projectDir: string, srcPath: string) =>
  invoke<BackgroundAssetInfo>("import_background_asset", { projectDir, srcPath })
```

### Inputs

- `projectDir` (`string`) - the recording's folder (the one holding `edit.json`).
- `srcPath` (`string`) - absolute path to the file the user picked. *Why a path rather than bytes:* the file can be gigabytes, and the Rust side copies it straight from disk to disk.

### Returns

`Promise<BackgroundAssetInfo>` - the copy that now lives inside the project. Rejects with a message naming the accepted extensions when the file is not png/jpg/jpeg/webp/gif/mp4/webm/mov, or is not a file at all.

### Used by

`BackgroundAssetCard` (`src/editor/panels/BackgroundAssetCard.tsx`) - the "Import image or video" tile.

## backgroundAssetInfo

```ts
export const backgroundAssetInfo = (projectDir: string, relPath: string) =>
  invoke<BackgroundAssetInfo | null>("background_asset_info", { projectDir, relPath })
```

What the panel shows for the asset already named in `edit.json`, since a reopened project carries only the relative path. `null` when the file is gone (project moved without its `background/` folder, file deleted outside the app), which is what lets the card say "File missing" rather than show a stale name. Also re-creates a thumbnail that went missing.

## removeBackgroundAsset

```ts
export const removeBackgroundAsset = (projectDir: string, relPath: string) =>
  invoke<void>("remove_background_asset", { projectDir, relPath })
```

Deletes the imported file and its thumbnail. Idempotent. It does NOT touch `edit.json`: the caller clears `background.asset` in its own `onSaveSettings`, because the doc lives in the frontend and a Rust-side write would be overwritten by the next `save_edit`.

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

## ProjectManifest

```ts
export interface ProjectManifest { version: number; created_unix_ms: number; source_w: number; source_h: number; app_version: string; preprocessed: boolean }
```

Mirrors the Rust `ProjectManifest`: the `project.tcursor` file written into a project folder at record/save time. `preprocessed` is read by `useEditorData` (via `getProjectManifest`) to decide whether to skip its own lazy `ensure_*` regeneration for an already-warmed project.

## getProjectManifest

```ts
export const getProjectManifest = (folder: string) => invoke<ProjectManifest>("get_project_manifest", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<ProjectManifest>` - never rejects; a missing or corrupt `project.tcursor` resolves to a synthesized "unknown source" default (mirrors the Rust `ProjectManifest::load_or_default`), with `preprocessed: false`.

### Used by

`useEditorData` (`src/editor/hooks/useEditorData.ts`) - fetched once per `[folder]` into `preprocessed`.

## preprocessProject

```ts
export const preprocessProject = (folder: string) => invoke<void>("preprocess_project", { folder })
```

### Inputs

- `folder` (`string`) - the just-finished (or reopened) project directory.

### Returns

`Promise<void>` that resolves as soon as the background pass has been *spawned*, not when it finishes - progress and completion arrive asynchronously via three Tauri events: `preprocess-progress` (payload `number`, 0..100), `preprocess-done` (payload the folder), and `preprocess-error` (payload a message).

### Used by

`useRecordingFlow` (`src/hud/hooks/useRecordingFlow.ts`) - called right after `stopRecording`/`webcam.stop()` resolve, awaited (via the events above) before `onEdit` opens the editor.

## openProject

```ts
export const openProject = () => invoke<string>("open_project")
```

### Returns

`Promise<string>` - the CONTAINING FOLDER of the `*.tcursor` file the user picked (the editor always opens a folder, never the manifest file itself). Rejects if the user cancels the dialog.

### Used by

`Hud` (`src/hud/Hud.tsx`, `openExistingProject`) - called from the "Open Project" titlebar button; the returned folder is passed to `onEdit`, the same callback `App` wires to `stopRecording`'s result.

## RecentProject

```ts
export interface RecentProject { folder: string; name: string; opened_unix_ms: number }
```

Mirrors the Rust `RecentProject`: one entry in the small "recently opened" list, persisted server-side alongside settings.

## listRecentProjects

```ts
export const listRecentProjects = () => invoke<RecentProject[]>("list_recent_projects")
```

### Returns

`Promise<RecentProject[]>` - most-recently-opened first.

### Implementation

Not yet consumed by any `.tsx` file (no recents UI exists yet); defined so the backend command surface is available when one is built.

## getLaunchProject

```ts
export const getLaunchProject = () => invoke<string | null>("get_launch_project")
```

### Returns

`Promise<string | null>` - the project folder to open if this process was just launched by double-clicking a `.tcursor` file (Windows file association), or `null` on a normal launch.

### Used by

`App` (`src/App.tsx`) - called once on mount; a non-null folder routes straight to the editor instead of showing the HUD. Only covers cold start - see the Rust `LaunchProject` doc comment for the warm-launch (already-running instance) follow-up.

## detectSilences

```ts
export const detectSilences = (folder: string) => Promise<[number, number][]>
```

Remove silences (`src-tauri/src/export/pipeline/silence.rs`): the recording's quiet stretches, silent on mic AND system when both exist, as clip-time spans already padded by 150 ms a side, at least 700 ms long and clamped into the trim. The editor applies them as one `add_cuts` so the batch is one undo step, skipping spans already inside a cut.
