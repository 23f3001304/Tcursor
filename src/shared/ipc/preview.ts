import { invoke } from "@tauri-apps/api/core";
import type { Arrangement } from "../edit";

export interface CamSample {
  t: number;
  scale: number;
  cx: number;
  cy: number;
  curx: number;
  cury: number;
}
export const cameraTrack = (folder: string) => invoke<CamSample[]>("camera_track", { folder });

export interface PreviewLayout {
  screen: [number, number, number, number];
  radius: number;
  cam: [number, number, number, number, number, number, number, number, number] | null;
  canvas: [number, number];
  screenAlpha?: number;
  camAlpha?: number;
  src?: [number, number, number, number];
}
export const previewLayout = (folder: string) => invoke<PreviewLayout>("preview_layout", { folder });

export interface PanelRectDto {
  rect: [number, number, number, number];
  radius: number;
  alpha: number;
  ring_px: number;
  ring_color: [number, number, number];
}

export interface LayoutPresetDto {
  screen: PanelRectDto;
  cam: PanelRectDto;
  arrangement: Arrangement;
}
export type LayoutPresetName = "screen" | "camera" | "presenter" | "screen_only" | "camera_only";

export interface SegRectDto {
  id: string;
  screen: PanelRectDto | null;
  cam: PanelRectDto | null;
}

export interface SourceSpanDto {
  start_ms: number;
  src: [number, number, number, number];
  transition_ms: number;
  fit: [number, number];
}
export interface LayoutPresets {
  screen: LayoutPresetDto;
  camera: LayoutPresetDto;
  presenter: LayoutPresetDto;
  screen_only: LayoutPresetDto;
  camera_only: LayoutPresetDto;
  segs: SegRectDto[];
  spans: SourceSpanDto[];
  inset_w: number;
}

export const previewLayouts = (folder: string) => invoke<LayoutPresets>("preview_layouts", { folder });

export interface ClickSample {
  t: number;
  x: number;
  y: number;
}
export const clickTrack = (folder: string) => invoke<ClickSample[]>("click_track", { folder });

export const previewBg = (folder: string) => invoke<string>("preview_bg", { folder });

export const previewFrame = (folder: string, outMs: number) =>
  invoke<string>("preview_frame", { folder, outMs });

export interface GradientStops {
  from: [number, number, number];
  mid: [number, number, number] | null;
  to: [number, number, number];
  angle_deg: number;
}
export interface BackgroundThumb {
  id: string;
  name: string;
  kind: "mesh" | "gradient";
  group: string;
  png_base64: string;
  gradient?: GradientStops;
}

export const backgroundThumbs = () => invoke<BackgroundThumb[]>("background_thumbs");

export interface FxOverlayParams {
  ow: number;
  oh: number;
  style: string;
  color: [number, number, number];
  intensity: number;
  hits: [number, number, number][];
  spotCx?: number;
  spotCy?: number;
  spotDim?: number;
  spotRadius?: number;
  spotFeather?: number;
  spotAlpha?: number;
  spotMode?: string;
  spotTint?: [number, number, number];
  spotT?: number;
  videoMode?: string;
  videoAlpha?: number;
  videoT?: number;
  camRect?: [number, number, number, number];
  camRadius?: number;
  dimCamera?: boolean;
}
export const previewFxOverlay = (p: FxOverlayParams) =>
  invoke<string>("preview_fx_overlay", {
    ow: p.ow,
    oh: p.oh,
    style: p.style,
    color: p.color,
    intensity: p.intensity,
    hits: p.hits,
    spotCx: p.spotCx ?? null,
    spotCy: p.spotCy ?? null,
    spotDim: p.spotDim ?? null,
    spotRadius: p.spotRadius ?? null,
    spotFeather: p.spotFeather ?? null,
    spotAlpha: p.spotAlpha ?? null,
    spotMode: p.spotMode ?? null,
    spotTint: p.spotTint ?? null,
    spotT: p.spotT ?? null,
    videoMode: p.videoMode ?? null,
    videoAlpha: p.videoAlpha ?? null,
    videoT: p.videoT ?? null,
    camRect: p.camRect ?? null,
    camRadius: p.camRadius ?? null,
    dimCamera: p.dimCamera ?? null,
  });
