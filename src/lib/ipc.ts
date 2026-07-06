import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { DisplayInfo, AudioInfo } from "../hud/devices/selectDevices";
import type { Settings } from "../hud/settings/settings";
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
export const saveWebcam = (folder: string, bytes: Uint8Array) =>
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
export interface PreviewLayout { screen: [number, number, number, number]; radius: number; cam: [number, number, number, number, number] | null; screenAlpha?: number; camAlpha?: number }
export const previewLayout = (folder: string) => invoke<PreviewLayout>("preview_layout", { folder });
/** One panel's rect (fraction of output, [x, y, w, h]) + corner radius (fraction of output width)
 *  + cross-dissolve alpha (0..1) - the same basis `PreviewLayout` uses. */
export interface PanelRectDto { rect: [number, number, number, number]; radius: number; alpha: number }
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
  });
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
