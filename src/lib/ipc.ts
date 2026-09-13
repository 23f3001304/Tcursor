import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { DisplayInfo, AudioInfo } from "../hud/devices/selectDevices";
import type { Settings } from "../hud/settings/settings";
import type { EditDoc, EditOp } from "./edit";

// Canvas-preview compositing IPC (camera curve, layout panel rects, click track, FX overlay) lives
// in its own file for size - re-exported here so every `from "../../lib/ipc"` import is unchanged.
export * from "./ipcPreview";

export const listDisplays = () => invoke<DisplayInfo[]>("list_displays");
export const listAudioInputs = () => invoke<AudioInfo[]>("list_audio_inputs");
export const startRecording = (projectName: string, micId: string | null, targetId: string | null, systemAudio: boolean, gameMode: boolean) =>
  invoke<string>("start_recording", { projectName, micId, targetId, systemAudio, gameMode });
export const pauseRecording = () => invoke<void>("pause_recording");
export const resumeRecording = () => invoke<void>("resume_recording");
export const stopRecording = () =>
  invoke<{ folder: string; frames: number }>("stop_recording");
export const saveWebcam = (folder: string, bytes: Uint8Array) =>
  invoke<void>("save_webcam", { folder, bytes });
export const appendWebcam = (folder: string, bytes: Uint8Array) =>
  invoke<void>("append_webcam", { folder, bytes });
/** Output frame SIZE (mirrors Rust `export::settings::Resolution`), independent of the doc's
 *  `Aspect` (the RATIO). Fixed presets name the SHORT edge in px - `"p1080"` on a landscape
 *  aspect is height=1080 (1920x1080); on a portrait aspect the short edge is the WIDTH
 *  (1080x1920). `"source"` (the default) keeps whatever the aspect alone resolves to. */
export type ExportResolution = "p720" | "p1080" | "p1440" | "p2160" | "source";
/** Output frame rate (mirrors Rust `export::settings::Fps`). `"source"` matches the capture/
 *  display refresh rate (capped at 60); `"f60"` (the default) is a fixed 60 regardless of it. */
export type ExportFps = "f30" | "f60" | "source";
/** Export container/codec (mirrors Rust `export::settings::Format`). `"gif"` cannot carry audio. */
export type ExportFormat = "mp4" | "webm" | "gif";
/** User-chosen export settings, collected by `ExportDialog` and sent to `export_project` -
 *  mirrors Rust `export::settings::ExportSettings`. `DEFAULT_EXPORT_SETTINGS` reproduces today's
 *  export exactly. */
export interface ExportSettings { resolution: ExportResolution; fps: ExportFps; quality_crf: number; format: ExportFormat }
/** `ExportSettings::default()` on the Rust side: Source resolution/fps-fallback semantics aside,
 *  this is the exact today's-export configuration (60fps, CRF 24, MP4/H.264). */
export const DEFAULT_EXPORT_SETTINGS: ExportSettings = { resolution: "source", fps: "f60", quality_crf: 24, format: "mp4" };
export const exportProject = (folder: string, settings: ExportSettings) =>
  invoke<void>("export_project", { folder, settings });
export const getSettings = () => invoke<Settings>("get_settings");
export const setSettings = (settings: Settings) => invoke<void>("set_settings", { settings });
export const getEdit = (folder: string) => invoke<EditDoc>("get_edit", { folder });
export const applyEditOp = (folder: string, op: EditOp) => invoke<EditDoc>("apply_edit_op", { folder, op });
export const saveEdit = (folder: string, doc: EditDoc) => invoke<void>("save_edit", { folder, doc });
/** One labeled step of the AI director's plan. */
export type AiStep = { op: EditOp; label: string };
/** The AI director's plan as ordered, labeled steps (NOT applied) - the editor reveals them
 *  one-by-one via `applyEditOp` for the agentic feel. */
export const aiPlan = (folder: string, model?: string) =>
  invoke<AiStep[]>("ai_plan", { folder, model });
/** Locally-installed Ollama model names, for the AI panel's Engine picker. Empty (never
 *  rejects) when Ollama isn't running. */
export const listOllamaModels = () => invoke<string[]>("list_ollama_models");
/** One cursor sprite (Capitaine pack) for the canvas preview: lowercase type name, a PNG data
 *  URL (cropped + dark-inverted like the export), the hotspot (0..1 of the cropped sprite), and
 *  the original canvas height for uniform scaling. */
export interface CursorSpriteDto { kind: string; url: string; hot: [number, number]; canvas_h: number }
/** How a pack animates its busy cursor (pack format v2), mirroring Rust `BusySpec`. `frames` is
 *  how many explicit `busy_NN.png` files the pack ships - `0` means `anim` synthesises the
 *  animation from the single `busy.png` instead. */
export interface BusySpecDto { anim: "spin" | "flip" | "pulse"; fps: number; frames: number }
/** The recording's selected cursor pack, ready to draw: one sprite per kind, the pack's explicit
 *  busy frames (empty unless it ships them), and its declared busy animation (null for the
 *  embedded set and any v1 pack). `busy` + `busy_frames` are what let the preview run the same
 *  `busyPose` the export does. */
export interface CursorPackDto {
  sprites: CursorSpriteDto[]; busy_frames: CursorSpriteDto[]; busy: BusySpecDto | null;
}
export const cursorSprites = (folder: string) => invoke<CursorPackDto>("cursor_sprites", { folder });
/** One cursor-shape change at output time `t` (ms); `kind` is the lowercase cursor-type name. */
export interface CursorKindSample { t: number; kind: string }
export const cursorKinds = (folder: string) => invoke<CursorKindSample[]>("cursor_kinds", { folder });
/** One captured OS cursor bitmap: its layer id, pixel size, hotspot in pixels, and the recorded
 *  PNG as a data URL. This is the real cursor that was on screen, not a sprite-pack stand-in. */
export interface CapturedCursorDto { id: number; w: number; h: number; hx: number; hy: number; url: string }
/** The recording's captured OS-cursor layer: the bitmaps plus `[t, id]` samples (output ms)
 *  saying which was showing, and the recorded video's own pixel size. The bitmaps are in SOURCE
 *  pixels, so `src_w` is what scales them relative to the screen content (`contentScale`); it is
 *  `0` only when the backend could not probe the video. `null` for a pre-layer recording. */
export interface CursorLayerDto {
  cursors: CapturedCursorDto[]; track: [number, number][]; src_w: number; src_h: number;
}
export const cursorLayer = (folder: string) => invoke<CursorLayerDto | null>("cursor_layer", { folder });
/** One selectable cursor pack: `id` persists into `CursorSettings.pack`, `name` is shown in the
 *  picker, `builtin` marks a pack the user cannot delete (the embedded set, always first, or one
 *  bundled with the app). `dir` is the pack's folder, so the grid loads each tile's sprite through
 *  the asset protocol rather than the backend base64ing every pack's nine PNGs into one reply.
 *  `files` maps each kind wire name to its filename inside `dir`, already alias-resolved (the
 *  embedded pack spells its arrow `pointer.png`) and already carrying the busy-is-arrow
 *  substitution, so the grid never has to know either rule; a kind the pack does not ship is
 *  absent. `busy` is the pack's busy animation, so a hovered tile previews it with the same
 *  `busyPose` the export runs. */
export interface CursorPackInfo {
  id: string; name: string; builtin: boolean; dir: string;
  files: Record<string, string>; busy: BusySpecDto | null;
}
/** Built-in pack first, then every imported pack under the app's cursors folder. */
export const listCursorPacks = () => invoke<CursorPackInfo[]>("list_cursor_packs");
/** Import a folder (arrow.png/ibeam.png/.../hotspots.json) as a new cursor pack; rejects if it
 *  has no recognized cursor PNGs. Returns the new pack so the caller can select it immediately. */
export const importCursorPack = (path: string) => invoke<CursorPackInfo>("import_cursor_pack", { path });

/** An imported background file (`settings::bg_asset::BackgroundAssetInfo`). `rel_path` is always
 *  relative to the project folder and forward-slashed (`background/<file>`) - what
 *  `settings.background.asset` stores, and the reason a project stays portable. `duration_ms` is
 *  null for a still; a video's is what the preview loops on. */
export interface BackgroundAssetInfo { rel_path: string; kind: "image" | "video"; width: number; height: number; duration_ms: number | null }
/** Copy the user's chosen image/video into `<project>/background/` (keeping its name, deduped),
 *  thumbnail it, and describe it. Rejects anything that is not png/jpg/jpeg/webp/gif/mp4/webm/mov. */
export const importBackgroundAsset = (projectDir: string, srcPath: string) =>
  invoke<BackgroundAssetInfo>("import_background_asset", { projectDir, srcPath });
/** What the panel shows for the asset already named in `edit.json`; `null` when the file is gone
 *  (project moved without its `background/` folder, file deleted outside the app). */
export const backgroundAssetInfo = (projectDir: string, relPath: string) =>
  invoke<BackgroundAssetInfo | null>("background_asset_info", { projectDir, relPath });

/** Remove silences: the recording's quiet stretches (mic AND system when both exist) as clip-time
 *  spans, padded and clamped into the trim, for one `add_cuts` (one undo step). */
export const detectSilences = (folder: string) => invoke<[number, number][]>("detect_silences", { folder });
/** Delete an imported background and its thumbnail. Does NOT touch `edit.json`: the caller clears
 *  `background.asset` in its own save, which is the only writer of the doc. */
export const removeBackgroundAsset = (projectDir: string, relPath: string) =>
  invoke<void>("remove_background_asset", { projectDir, relPath });
/** Filmstrip thumbnail file paths (wrap each with `fileSrc`); one cached ffmpeg pass. */
export const ensureThumbs = (folder: string, count: number) => invoke<string[]>("ensure_thumbs", { folder, count });
/** A cached waveform PNG path for the system or mic track ("" if that source wasn't recorded). */
export const ensureWaveform = (folder: string, which: "system" | "mic") => invoke<string>("ensure_waveform", { folder, which });
/** A cached mixed (mic+system) preview-audio file path so the editor can play sound. */
export const ensurePreviewAudio = (folder: string) => invoke<string>("ensure_preview_audio", { folder });
/** Transcode (once, cached) a low-res preview proxy at `height` px; returns its path. */
export const ensureProxy = (folder: string, height: number) => invoke<string>("ensure_proxy", { folder, height });
/** Proxy height `preprocess_project` generates (mirrors Rust `preprocess::DEFAULT_PROXY_HEIGHT`)
 *  - also `Editor.tsx`'s initial `quality` state, so a freshly preprocessed project's default
 *  quality always matches what preprocessing already put on disk. */
export const DEFAULT_PROXY_HEIGHT = 720;
/** Asset-protocol URL for a local file path, for a native <video> element. */
export const fileSrc = (path: string) => convertFileSrc(path);
export const setCapturable = (capturable: boolean) =>
  invoke<boolean>("set_capturable", { capturable });
/** The `project.tcursor` manifest written into a project folder at record/save time (mirrors the
 *  Rust `ProjectManifest`). `preprocessed` is read by `useEditorData` (via `getProjectManifest`)
 *  to decide whether to skip its own lazy `ensure_*` calls. */
export interface ProjectManifest {
  version: number; created_unix_ms: number; source_w: number; source_h: number;
  app_version: string; preprocessed: boolean;
}
/** Reads the `project.tcursor` manifest for `folder` - a synthesized "unknown source" default
 *  (never a rejection) when it is missing or corrupt, mirroring the Rust
 *  `ProjectManifest::load_or_default`. */
export const getProjectManifest = (folder: string) => invoke<ProjectManifest>("get_project_manifest", { folder });
/** Whether this recording's video already has the OS cursor baked into its pixels - derived from
 *  the immutable record-time `settings.json` snapshot, NOT from the editable doc. `false` means
 *  the "System" cursor style must be re-created from the recorded path (see `cursorPreview.ts`).
 *  Resolves `true` (today's behavior: draw nothing) if the snapshot is missing or unreadable. */
export const osCursorInVideo = (folder: string) => invoke<boolean>("os_cursor_in_video", { folder });
/** Kicks off the full editor-preview preprocessing pass - proxy (`DEFAULT_PROXY_HEIGHT`), filmstrip
 *  thumbnails, system+mic waveforms, mixed preview audio, and the `edit.json` seed - on a
 *  background thread, reusing the exact `ensure_*`/seed functions the editor's own lazy fallback
 *  calls. The returned promise resolves as soon as the background thread is spawned, NOT when
 *  preprocessing finishes - the caller awaits completion via three events instead: `preprocess-
 *  progress` (payload `number`, 0..100), `preprocess-done` (payload the folder), and
 *  `preprocess-error` (payload a message). */
export const preprocessProject = (folder: string) => invoke<void>("preprocess_project", { folder });
/** Opens a native file-picker filtered to `*.tcursor`, resolves the picked file to its
 *  CONTAINING FOLDER (the editor always opens a folder, never the manifest file itself), and
 *  records it in the recents list. Rejects if the user cancels the dialog. */
export const openProject = () => invoke<string>("open_project");
/** One entry in the small "recently opened" list (persisted alongside settings in the config
 *  dir), most-recently-opened first. */
export interface RecentProject { folder: string; name: string; opened_unix_ms: number }
/** For a future recents UI; not yet rendered anywhere. */
export const listRecentProjects = () => invoke<RecentProject[]>("list_recent_projects");
/** The cold-start file-association target, if the app was just launched by double-clicking a
 *  `.tcursor` file (`null` on a normal launch). `App` calls this once on mount to decide whether
 *  to route straight to the editor instead of showing the HUD. Warm-launch (the app already
 *  running) is not covered - see the Rust `LaunchProject` doc comment. */
export const getLaunchProject = () => invoke<string | null>("get_launch_project");
