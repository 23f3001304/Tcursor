import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { DisplayInfo, AudioInfo } from "../hud/selectDevices";
import type { Settings } from "../hud/settings";
import type { EditDoc, EditOp } from "./edit";

/** One camera-curve sample: output time (ms), the zoom as scale + center, and the cursor
 *  position - cx/cy/curx/cury are 0..1 fractions of the screen content. */
export interface CamSample { t: number; scale: number; cx: number; cy: number; curx: number; cury: number }

export const listDisplays = () => invoke<DisplayInfo[]>("list_displays");
export const listAudioInputs = () => invoke<AudioInfo[]>("list_audio_inputs");
export const startRecording = (projectName: string, micId: string | null, systemAudio: boolean, gameMode: boolean) =>
  invoke<void>("start_recording", { projectName, micId, systemAudio, gameMode });
export const pauseRecording = () => invoke<void>("pause_recording");
export const resumeRecording = () => invoke<void>("resume_recording");
export const stopRecording = () =>
  invoke<{ folder: string; frames: number }>("stop_recording");
export const saveWebcam = (folder: string, bytes: number[]) =>
  invoke<void>("save_webcam", { folder, bytes });
export const exportProject = (folder: string) =>
  invoke<void>("export_project", { folder });
export const getSettings = () => invoke<Settings>("get_settings");
export const setSettings = (settings: Settings) => invoke<void>("set_settings", { settings });
export const getEdit = (folder: string) => invoke<EditDoc>("get_edit", { folder });
export const applyEditOp = (folder: string, op: EditOp) => invoke<EditDoc>("apply_edit_op", { folder, op });
export const saveEdit = (folder: string, doc: EditDoc) => invoke<void>("save_edit", { folder, doc });
export const aiAutoedit = (folder: string, model?: string) =>
  invoke<EditDoc>("ai_autoedit", { folder, model });
export const cameraTrack = (folder: string) => invoke<CamSample[]>("camera_track", { folder });
/** The static export layout: screen rect + corner radius + webcam rect, as fractions of the
 *  output, so the canvas preview frames the screen + webcam from the export layout (not a guess). */
export interface PreviewLayout { screen: [number, number, number, number]; radius: number; cam: [number, number, number, number, number] | null }
export const previewLayout = (folder: string) => invoke<PreviewLayout>("preview_layout", { folder });
/** One click ripple: output time (ms) + 0..1 screen-content position (same basis as CamSample's cursor). */
export interface ClickSample { t: number; x: number; y: number }
export const clickTrack = (folder: string) => invoke<ClickSample[]>("click_track", { folder });
/** One recorded effect-hold interval in output time (ms) - e.g. a spotlight held via hotkey while recording. */
export interface HoldSpan { start_ms: number; end_ms: number }
/** The recorded spotlight-hold intervals, so the editor preview lights held spotlights like the export. */
export const spotlightHolds = (folder: string) => invoke<HoldSpan[]>("spotlight_holds", { folder });
/** The export background (mesh/gradient) as a PNG data URL, so the canvas preview matches the export. */
export const previewBg = (folder: string) => invoke<string>("preview_bg", { folder });
/** One cursor sprite (Capitaine pack) for the canvas preview: lowercase type name, a PNG data
 *  URL (cropped + dark-inverted like the export), the hotspot (0..1 of the cropped sprite), and
 *  the original canvas height for uniform scaling. */
export interface CursorSpriteDto { kind: string; url: string; hot: [number, number]; canvas_h: number }
export const cursorSprites = (folder: string) => invoke<CursorSpriteDto[]>("cursor_sprites", { folder });
/** One cursor-shape change at output time `t` (ms); `kind` is the lowercase cursor-type name. */
export interface CursorKindSample { t: number; kind: string }
export const cursorKinds = (folder: string) => invoke<CursorKindSample[]>("cursor_kinds", { folder });
/** Filmstrip thumbnail file paths (wrap each with `fileSrc`); one cached ffmpeg pass. */
export const ensureThumbs = (folder: string, count: number) => invoke<string[]>("ensure_thumbs", { folder, count });
/** A cached waveform PNG path for the system or mic track ("" if that source wasn't recorded). */
export const ensureWaveform = (folder: string, which: "system" | "mic") => invoke<string>("ensure_waveform", { folder, which });
/** A cached mixed (mic+system) preview-audio file path so the editor can play sound. */
export const ensurePreviewAudio = (folder: string) => invoke<string>("ensure_preview_audio", { folder });
/** Transcode (once, cached) a low-res preview proxy at `height` px; returns its path. */
export const ensureProxy = (folder: string, height: number) => invoke<string>("ensure_proxy", { folder, height });
/** Asset-protocol URL for a local file path, for a native <video> element. */
export const fileSrc = (path: string) => convertFileSrc(path);
export const setCapturable = (capturable: boolean) =>
  invoke<boolean>("set_capturable", { capturable });
