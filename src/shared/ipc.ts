import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { DisplayInfo, AudioInfo } from "../hud/devices/selectDevices";
import type { Settings } from "../hud/settings/settings";
import type { EditDoc, EditOp } from "./edit";
import type { AiRun } from "./aiRun";

export const listDisplays = () => invoke<DisplayInfo[]>("list_displays");
export const listAudioInputs = () => invoke<AudioInfo[]>("list_audio_inputs");
export const startRecording = (
  projectName: string,
  micId: string | null,
  targetId: string | null,
  systemAudio: boolean,
  gameMode: boolean,
) => invoke<string>("start_recording", { projectName, micId, targetId, systemAudio, gameMode });
export const pauseRecording = () => invoke<void>("pause_recording");
export const resumeRecording = () => invoke<void>("resume_recording");
export const stopRecording = () => invoke<{ folder: string; frames: number }>("stop_recording");

export const appendWebcam = (folder: string, bytes: Uint8Array, segment = 1) =>
  invoke<void>("append_webcam", { folder, bytes, segment });

export const markWebcamSegment = (segment: number) => invoke<void>("mark_webcam_segment", { segment });

export const switchMic = (deviceId: string | null) => invoke<void>("switch_mic", { deviceId });

export const switchDisplay = (targetId: string) => invoke<void>("switch_display", { targetId });

export type ExportResolution = "p720" | "p1080" | "p1440" | "p2160" | "source";

export type ExportFps = "f30" | "f60" | "source";

export type ExportFormat = "mp4" | "webm" | "gif";

export interface ExportSettings {
  resolution: ExportResolution;
  fps: ExportFps;
  quality_crf: number;
  format: ExportFormat;
}

export const DEFAULT_EXPORT_SETTINGS: ExportSettings = {
  resolution: "source",
  fps: "f60",
  quality_crf: 24,
  format: "mp4",
};
export const exportProject = (folder: string, settings: ExportSettings) =>
  invoke<void>("export_project", { folder, settings });
export const getSettings = () => invoke<Settings>("get_settings");
export const setSettings = (settings: Settings) => invoke<void>("set_settings", { settings });
export const getEdit = (folder: string) => invoke<EditDoc>("get_edit", { folder });
export const applyEditOp = (folder: string, op: EditOp) => invoke<EditDoc>("apply_edit_op", { folder, op });
export const saveEdit = (folder: string, doc: EditDoc) => invoke<void>("save_edit", { folder, doc });

export interface OllamaModel {
  name: string;
  vision: boolean;
}

export const listOllamaModels = () => invoke<OllamaModel[]>("list_ollama_models");

export const aiPropose = (folder: string, model?: string) => invoke<AiRun>("ai_propose", { folder, model });

export interface CursorSpriteDto {
  kind: string;
  url: string;
  hot: [number, number];
  canvas_h: number;
}

export interface BusySpecDto {
  anim: "spin" | "flip" | "pulse";
  fps: number;
  frames: number;
}

export interface CursorPackDto {
  sprites: CursorSpriteDto[];
  busy_frames: CursorSpriteDto[];
  busy: BusySpecDto | null;
  material: string | null;
}
export const cursorSprites = (folder: string) => invoke<CursorPackDto>("cursor_sprites", { folder });

export interface CursorKindSample {
  t: number;
  kind: string;
}
export const cursorKinds = (folder: string) => invoke<CursorKindSample[]>("cursor_kinds", { folder });

export interface CapturedCursorDto {
  id: number;
  w: number;
  h: number;
  hx: number;
  hy: number;
  url: string;
}

export interface CursorLayerDto {
  cursors: CapturedCursorDto[];
  track: [number, number][];
  src_w: number;
  src_h: number;
}
export const cursorLayer = (folder: string) => invoke<CursorLayerDto | null>("cursor_layer", { folder });

export interface CursorPackInfo {
  id: string;
  name: string;
  category: string;
  builtin: boolean;
  dir: string;
  files: Record<string, string>;
  busy: BusySpecDto | null;
  material: string | null;
}

export const listCursorPacks = () => invoke<CursorPackInfo[]>("list_cursor_packs");

export const importCursorPack = (path: string) => invoke<CursorPackInfo>("import_cursor_pack", { path });

export const createPackTemplate = (dir: string) => invoke<string>("create_pack_template", { dir });

export interface BackgroundAssetInfo {
  rel_path: string;
  kind: "image" | "video";
  width: number;
  height: number;
  duration_ms: number | null;
}

export const importBackgroundAsset = (projectDir: string, srcPath: string) =>
  invoke<BackgroundAssetInfo>("import_background_asset", { projectDir, srcPath });

export const backgroundAssetInfo = (projectDir: string, relPath: string) =>
  invoke<BackgroundAssetInfo | null>("background_asset_info", { projectDir, relPath });

export const detectSilences = (folder: string) => invoke<[number, number][]>("detect_silences", { folder });

export const removeBackgroundAsset = (projectDir: string, relPath: string) =>
  invoke<void>("remove_background_asset", { projectDir, relPath });

export const ensureThumbs = (folder: string, count: number, height: number) =>
  invoke<string[]>("ensure_thumbs", { folder, count, height });

export const ensureWaveform = (folder: string, which: "system" | "mic") =>
  invoke<string>("ensure_waveform", { folder, which });

export const ensurePreviewAudio = (folder: string) => invoke<string>("ensure_preview_audio", { folder });

export const ensureProxy = (folder: string, height: number) =>
  invoke<string>("ensure_proxy", { folder, height });

export const DEFAULT_PROXY_HEIGHT = 720;

export const fileSrc = (path: string) => convertFileSrc(path);
export const setCapturable = (capturable: boolean) => invoke<boolean>("set_capturable", { capturable });

export interface ProjectManifest {
  version: number;
  created_unix_ms: number;
  source_w: number;
  source_h: number;
  app_version: string;
  preprocessed: boolean;
}

export const getProjectManifest = (folder: string) =>
  invoke<ProjectManifest>("get_project_manifest", { folder });

export const osCursorInVideo = (folder: string) => invoke<boolean>("os_cursor_in_video", { folder });

export const preprocessProject = (folder: string) => invoke<void>("preprocess_project", { folder });

export const openProject = () => invoke<string>("open_project");

export interface RecentProject {
  folder: string;
  name: string;
  opened_unix_ms: number;
}

export const listRecentProjects = () => invoke<RecentProject[]>("list_recent_projects");

export const getLaunchProject = () => invoke<string | null>("get_launch_project");

export * from "./ipc/preview";
export * from "./ipc/asr";
