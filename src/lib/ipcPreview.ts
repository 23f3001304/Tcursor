// Canvas-preview compositing IPC: the camera curve, the resolved layout panel rects (presets +
// per-segment arrangements), the click track, and the FX-overlay render request - everything the
// editor's canvas preview needs to reproduce the export's framing/zoom/FX pixel-for-pixel. Split
// out of ipc.ts (T34) purely for file size - this is a self-contained domain (nothing here is used
// outside `editor/stage`/`editor/timeline`), re-exported from ipc.ts so every existing
// `from "../../lib/ipc"` import keeps working unchanged.
import { invoke } from "@tauri-apps/api/core";
import type { Arrangement } from "./edit";

/** One camera-curve sample: output time (ms), the zoom as scale + center, and the cursor
 *  position - cx/cy/curx/cury are 0..1 fractions of the screen content. */
export interface CamSample { t: number; scale: number; cx: number; cy: number; curx: number; cury: number }
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
/** One layout preset's two panels: screen (zoomed base layer) + cam (fixed top layer), plus the
 *  same preset expressed as POSES (`arrangement`) - what a `set_arrangement` op is fed to turn this
 *  preset into a directly manipulable arrangement. Derived in Rust
 *  (`scene::arrangement::arrangement_of_preset`) so the conversion has exactly one definition;
 *  resolving it back reproduces this preset's pixels (proven to 0.5px by the Rust parity test). */
export interface LayoutPresetDto { screen: PanelRectDto; cam: PanelRectDto; arrangement: Arrangement }
export type LayoutPresetName = "screen" | "camera" | "presenter" | "screen_only" | "camera_only";
/** One `EditDoc.layout` segment's own resolved panels, by id (T34 `preview_layouts` extension).
 *  `null` on a field means "this segment doesn't override that panel - fall back to its `layout`
 *  preset", which is both fields on a segment with no `arrangement` at all; a segment WITH one
 *  always resolves both (never a mix), since Rust's `resolve_arrangement` always returns a full
 *  scene - a panel the arrangement hides still gets a real rect, just `alpha: 0`. Resolved in Rust
 *  through the exact function the export's `LayoutTrack` uses per segment
 *  (`scene::layout::resolve_seg_scene`) - the TS side never re-derives a pose, only picks which
 *  already-resolved rect to show (see `layoutTrack.ts`'s `layoutAt`). */
export interface SegRectDto { id: string; screen: PanelRectDto | null; cam: PanelRectDto | null }
export interface LayoutPresets {
  screen: LayoutPresetDto; camera: LayoutPresetDto; presenter: LayoutPresetDto;
  screen_only: LayoutPresetDto; camera_only: LayoutPresetDto;
  /** One entry per `EditDoc.layout` segment, in doc order - not just posed ones, so a lookup by id
   *  is a single flat scan with no special-casing "this segment was never in the list". */
  segs: SegRectDto[];
  /** `FrameRenderer::inset_w_frac` - fraction of output width. The reference width the export's
   *  synthetic cursor scales against (`cursorset::draw`'s `panel` factor), NOT one of the panel
   *  rects above: a fixed baseline independent of the active preset/arrangement. `cursorPanel.ts`'s
   *  `panelFactor` divides the active screen panel's own width by this to shrink the preview cursor
   *  exactly like the export shrinks it for a narrow custom-arrangement panel. */
  inset_w: number;
}
/** All 5 layout presets' panel rects + alpha, plus each doc segment's own resolved rects when it
 *  carries an arrangement (`segs`), in one call - so the editor preview can cross-fade between
 *  layout segments itself (mirroring the export's LayoutTrack, posed or not) instead of only ever
 *  showing a segment's provenance preset. */
export const previewLayouts = (folder: string) => invoke<LayoutPresets>("preview_layouts", { folder });

/** One click ripple: output time (ms) + 0..1 screen-content position (same basis as CamSample's cursor). */
export interface ClickSample { t: number; x: number; y: number }
export const clickTrack = (folder: string) => invoke<ClickSample[]>("click_track", { folder });

/** The export background (mesh/gradient) as a PNG data URL, so the canvas preview matches the export. */
export const previewBg = (folder: string) => invoke<string>("preview_bg", { folder });

/** One background-picker tile: a 96x54 thumbnail rendered by the EXPORT's own background code,
 *  so a tile is a true miniature of what picking it produces. `kind` is the `BackgroundKind` the
 *  tile applies, `group` the section it belongs to; `id` is what gets written to `background.mesh`
 *  for a wallpaper, while a gradient tile instead carries the `gradient` stops the panel applies
 *  (the presets live in Rust,
 *  `settings::wallpapers::GRADIENT_WALLPAPERS`, so there is no second copy to drift). `png_base64`
 *  is empty when the decode failed (no ffmpeg) - the tile stays selectable and the panel draws a
 *  plain swatch for it. */
export interface GradientStops { from: [number, number, number]; mid: [number, number, number] | null; to: [number, number, number]; angle_deg: number }
export interface BackgroundThumb { id: string; name: string; kind: "mesh" | "gradient"; group: string; png_base64: string; gradient?: GradientStops }
/** Every bundled wallpaper then every gradient preset, thumbnailed. Rendered once per process in
 *  Rust, so calling this per panel mount is cheap after the first time. Wallpapers arrive grouped
 *  (`group`: Ribbons, Folds, Scenic) in the order the picker shows them; gradients are one group. */
export const backgroundThumbs = () => invoke<BackgroundThumb[]>("background_thumbs");

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
