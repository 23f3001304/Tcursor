import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { DisplayInfo, AudioInfo } from "../hud/devices/selectDevices";
import type { Settings } from "../hud/settings/settings";
import type { EditDoc, EditOp } from "./edit";

/** One camera-curve sample: output time (ms), the zoom as scale + center, and the cursor
 *  position - cx/cy/curx/cury are 0..1 fractions of the screen content. */
export interface CamSample { t: number; scale: number; cx: number; cy: number; curx: number; cury: number }

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
export const aiAutoedit = (folder: string, model?: string) =>
  invoke<EditDoc>("ai_autoedit", { folder, model });
/** Locally-installed Ollama model names, for the AI panel's Engine picker. Empty (never
 *  rejects) when Ollama isn't running. */
export const listOllamaModels = () => invoke<string[]>("list_ollama_models");
export const cameraTrack = (folder: string) => invoke<CamSample[]>("camera_track", { folder });
/** The static export layout: screen rect + corner radius + webcam rect, as fractions of the
 *  output, so the canvas preview frames the screen + webcam from the export layout (not a guess).
 *  `cam`'s last 4 entries are the webcam ring: width (fraction of output width, 0 = no ring) then
 *  RGB 0..255 - mirrors the export's `Panel.ring_px`/`ring_color` riding alongside rect/radius.
 *  `canvas` is the resolved preview frame's pixel dimensions (follows `EditDoc.aspect`), so the
 *  editor sizes its canvas + `.e-stage` aspect-ratio from this instead of a hardcoded 16:9. */
export interface PreviewLayout { screen: [number, number, number, number]; radius: number; cam: [number, number, number, number, number, number, number, number, number] | null; canvas: [number, number]; screenAlpha?: number; camAlpha?: number }
export const previewLayout = (folder: string) => invoke<PreviewLayout>("preview_layout", { folder });
/** One panel's rect (fraction of output, [x, y, w, h]) + corner radius (fraction of output width)
 *  + cross-dissolve alpha (0..1) + ring width (fraction of output width, 0 = no ring) + ring color
 *  (RGB 0..255) - the same basis `PreviewLayout` uses. */
export interface PanelRectDto { rect: [number, number, number, number]; radius: number; alpha: number; ring_px: number; ring_color: [number, number, number] }
/** One layout preset's two panels: screen (zoomed base layer) + cam (fixed top layer). */
export interface LayoutPresetDto { screen: PanelRectDto; cam: PanelRectDto }
export type LayoutPresetName = "screen" | "camera" | "presenter" | "screen_only" | "camera_only";
export type LayoutPresets = Record<LayoutPresetName, LayoutPresetDto>;
/** All 5 layout presets' panel rects + alpha in one call, so the editor preview can cross-fade
 *  between layout presets itself (mirroring the export's LayoutTrack) instead of only ever
 *  showing the single static layout `previewLayout` returns. */
export const previewLayouts = (folder: string) => invoke<LayoutPresets>("preview_layouts", { folder });
/** One click ripple: output time (ms) + 0..1 screen-content position (same basis as CamSample's cursor). */
export interface ClickSample { t: number; x: number; y: number }
export const clickTrack = (folder: string) => invoke<ClickSample[]>("click_track", { folder });
/** The export background (mesh/gradient) as a PNG data URL, so the canvas preview matches the export. */
export const previewBg = (folder: string) => invoke<string>("preview_bg", { folder });
/** Render the FX overlay (spotlight + click effects) using the exact export shaders.
 *  Returns a PNG data URL of the overlay to composite on the preview canvas. */
export interface FxOverlayParams {
  ow: number; oh: number;
  style: string; color: [number, number, number]; intensity: number;
  hits: [number, number, number][];
  spotCx?: number; spotCy?: number; spotDim?: number;
  spotRadius?: number; spotFeather?: number; spotAlpha?: number;
  spotMode?: string; spotTint?: [number, number, number]; spotT?: number;
  videoMode?: string; videoAlpha?: number; videoT?: number;
  // Camera PiP exclusion (FX-render px): min_x, min_y, max_x, max_y + corner radius, and
  // whether to keep it lit at all - mirrors export's Spot.cam_rect/cam_radius/dim_camera.
  camRect?: [number, number, number, number]; camRadius?: number; dimCamera?: boolean;
}
export const previewFxOverlay = (p: FxOverlayParams) =>
  invoke<string>("preview_fx_overlay", {
    ow: p.ow, oh: p.oh, style: p.style, color: p.color, intensity: p.intensity,
    hits: p.hits,
    spotCx: p.spotCx ?? null, spotCy: p.spotCy ?? null, spotDim: p.spotDim ?? null,
    spotRadius: p.spotRadius ?? null, spotFeather: p.spotFeather ?? null,
    spotAlpha: p.spotAlpha ?? null, spotMode: p.spotMode ?? null,
    spotTint: p.spotTint ?? null, spotT: p.spotT ?? null,
    videoMode: p.videoMode ?? null, videoAlpha: p.videoAlpha ?? null,
    videoT: p.videoT ?? null,
    camRect: p.camRect ?? null, camRadius: p.camRadius ?? null, dimCamera: p.dimCamera ?? null,
  });
/** One cursor sprite (Capitaine pack) for the canvas preview: lowercase type name, a PNG data
 *  URL (cropped + dark-inverted like the export), the hotspot (0..1 of the cropped sprite), and
 *  the original canvas height for uniform scaling. */
export interface CursorSpriteDto { kind: string; url: string; hot: [number, number]; canvas_h: number }
export const cursorSprites = (folder: string) => invoke<CursorSpriteDto[]>("cursor_sprites", { folder });
/** One cursor-shape change at output time `t` (ms); `kind` is the lowercase cursor-type name. */
export interface CursorKindSample { t: number; kind: string }
export const cursorKinds = (folder: string) => invoke<CursorKindSample[]>("cursor_kinds", { folder });
/** One selectable cursor pack: `id` persists into `CursorSettings.pack`, `name` is shown in the
 *  picker, `builtin` marks the embedded set (not stored on disk, always first in the list). */
export interface CursorPackInfo { id: string; name: string; builtin: boolean }
/** Built-in pack first, then every imported pack under the app's cursors folder. */
export const listCursorPacks = () => invoke<CursorPackInfo[]>("list_cursor_packs");
/** Import a folder (arrow.png/ibeam.png/.../hotspots.json) as a new cursor pack; rejects if it
 *  has no recognized cursor PNGs. Returns the new pack so the caller can select it immediately. */
export const importCursorPack = (path: string) => invoke<CursorPackInfo>("import_cursor_pack", { path });
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
