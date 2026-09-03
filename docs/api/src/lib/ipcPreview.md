# src/lib/ipcPreview.ts

Canvas-preview compositing IPC: typed wrappers over Tauri's `invoke` for the camera curve, the resolved layout panel rects (presets + per-segment arrangements), the click track, the export background, and the FX-overlay render request - everything the editor's canvas preview needs to reproduce the export's framing/zoom/FX pixel-for-pixel. Split out of `ipc.ts` (T34) purely for file size; every existing `from "../../lib/ipc"` import keeps working because `ipc.ts` re-exports this whole module (`export * from "./ipcPreview"`).

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
export interface PreviewLayout { screen: [number, number, number, number]; radius: number; cam: [number, number, number, number, number, number, number, number, number] | null; canvas: [number, number]; screenAlpha?: number; camAlpha?: number }
```

The static export framing as fractions of the output (mirrors the Rust `PreviewLayout`): `screen` is the screen rect `[x, y, w, h]`, `radius` the corner radius (fraction of width), and `cam` the webcam PiP rect+ring `[x, y, w, h, radius, ringPx, ringR, ringG, ringB]` or `null` when hidden - `ringPx` is a fraction of output width (0 = no ring) and `ringR/G/B` are 0..255, mirroring the export's `Panel.ring_px`/`ring_color` riding alongside the rect/radius. `canvas` is the resolved preview frame's pixel dimensions `[w, h]` (follows `EditDoc.aspect`, via `Layout::resolve` on the Rust side) - `Stage.tsx` sizes its `<canvas>` and `.e-stage`'s aspect-ratio from this instead of a hardcoded 16:9. The canvas compositor frames the screen and webcam from this so the preview matches the export.

## previewLayout

```ts
export const previewLayout = (folder: string) => invoke<PreviewLayout>("preview_layout", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<PreviewLayout>` - the screen/webcam framing fractions for the recording.

## PanelRectDto

```ts
export interface PanelRectDto { rect: [number, number, number, number]; radius: number; alpha: number; ring_px: number; ring_color: [number, number, number] }
```

One panel's rect (fraction of output, `[x, y, w, h]`) + corner radius (fraction of output width) + cross-dissolve `alpha` (0..1) + ring width (fraction of output width, 0 = no ring) + ring color (RGB 0..255) - the same basis `PreviewLayout` uses.

### Used by

- `src/lib/ipcPreview.ts` - field of `LayoutPresetDto`, and (as `| null`) of `SegRectDto`.
- `src/editor/timeline/layoutTrack.ts` - `lerpRect` cross-fades between two `PanelRectDto`s.

## LayoutPresetDto

```ts
export interface LayoutPresetDto { screen: PanelRectDto; cam: PanelRectDto; arrangement: Arrangement }
```

One layout preset's two panels: `screen` (the zoomed base layer) + `cam` (the fixed top layer), plus the same preset expressed as POSES.

- `arrangement: Arrangement` - this preset as a pose pair, ready to feed a `set_arrangement` op so a segment based on the preset becomes directly manipulable. *Why it comes from the backend rather than being computed here from `screen`/`cam`:* resolving it back must reproduce the preset's own pixels, and Rust owns that geometry (`scene::arrangement::arrangement_of_preset`, proven to 0.5px by its parity test). A hidden panel reports `null`, matching the `Arrangement` rule.

### Used by

- `src/lib/ipcPreview.ts` - value type of `LayoutPresets`.
- `src/editor/timeline/layoutTrack.ts` - `presetOf` looks up the preset for a given `layout` name; `resolvedPanelsFor` falls back to it per panel when a segment has no `segs` override (it reads only `screen`/`cam`, never `arrangement`).

## LayoutPresetName

```ts
export type LayoutPresetName = "screen" | "camera" | "presenter" | "screen_only" | "camera_only";
```

The 5 layout preset names (matches `LayoutSeg.layout`'s known values).

### Used by

- `src/lib/ipcPreview.ts` - type of the 5 named preset fields on `LayoutPresets`.
- `src/editor/timeline/layoutTrack.ts` - `KNOWN` validates a `LayoutSeg.layout` string against this set, falling back to `"screen"` for an unrecognized name.

## SegRectDto

```ts
export interface SegRectDto { id: string; screen: PanelRectDto | null; cam: PanelRectDto | null }
```

One `EditDoc.layout` segment's own resolved panels, by id (T34 L2). `null` on a field means "this segment doesn't override that panel - fall back to its `layout` preset", which is both fields on a segment with no `arrangement` at all; a segment WITH one always resolves both (never a mix), since Rust's `resolve_arrangement` always returns a full scene - a panel the arrangement hides still gets a real rect, just `alpha: 0`. Resolved in Rust through the exact function the export's `LayoutTrack` uses per segment (`scene::layout::resolve_seg_scene`) - the TS side never re-derives a pose, only picks which already-resolved rect to show.

### Used by

- `src/lib/ipcPreview.ts` - element type of `LayoutPresets.segs`.
- `src/editor/timeline/layoutTrack.ts` - `resolvedPanelsFor` looks a segment's entry up by id, falling back to its preset per panel.

## LayoutPresets

```ts
export interface LayoutPresets {
  screen: LayoutPresetDto; camera: LayoutPresetDto; presenter: LayoutPresetDto;
  screen_only: LayoutPresetDto; camera_only: LayoutPresetDto;
  segs: SegRectDto[];
  inset_w: number;
}
```

All 5 layout presets' panel rects, keyed by name, plus `segs` - one entry per `EditDoc.layout` segment (T34 L2), in doc order, not just posed ones. An explicit interface rather than `Record<LayoutPresetName, LayoutPresetDto>` (its pre-T34-L2 shape) because `segs` isn't a `LayoutPresetDto`; indexing by `LayoutPresetName` (`presetOf`'s `presets[name]`) still type-checks, since that type is a union of literal keys this interface actually declares.

`inset_w` (fraction of output width, mirrors `FrameRenderer::inset_w_frac`) is the export's fixed reference width for the synthetic cursor's scale-down (`cursorset::draw`'s `panel` factor) - a baseline independent of the active preset/arrangement, NOT one of the panel rects above. `cursorPanel.ts`'s `panelFactor` divides the active screen panel's own width by this so the preview cursor shrinks exactly like the export shrinks it for a narrow custom-arrangement panel.

### Used by

- `src/editor/hooks/useEditorData.ts` - fetched via `previewLayouts` into state, passed down to `Stage`.
- `src/editor/timeline/layoutTrack.ts` - `layoutAt` cross-fades between presets AND per-segment overrides as the playhead crosses `LayoutSeg` boundaries.
- `src/editor/hooks/useCompositeLoop.ts` - held in a ref so the per-frame compositing loop can resolve the current layout (and `inset_w`, for the cursor's panel scale/clip) without waiting on React state.

## previewLayouts

```ts
export const previewLayouts = (folder: string) => invoke<LayoutPresets>("preview_layouts", { folder })
```

All 5 layout presets' panel rects + alpha, plus each doc segment's own resolved rects (`segs`) for a segment carrying a T34 arrangement, plus `inset_w`, in one call - so the editor preview can cross-fade between layout segments AND mirror the export's cursor panel scale/clip itself, instead of only ever showing a segment's provenance preset.

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<LayoutPresets>` - the 5 presets, keyed by name, plus `segs` and `inset_w`.

### Used by

`useEditorData` (`src/editor/hooks/useEditorData.ts`) - fetched on `[folder, rev]` (a layout edit, INCLUDING a `set_arrangement`/`clear_arrangement` op, is one of the things `rev` bumps for - see `Editor.tsx`'s `applyOp`, which bumps `rev` for every op except one ending in `_effect`).

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

## FxOverlayParams

```ts
export interface FxOverlayParams {
  ow: number; oh: number;
  style: string; color: [number, number, number]; intensity: number;
  hits: [number, number, number][];
  spotCx?: number; spotCy?: number; spotDim?: number;
  spotRadius?: number; spotFeather?: number; spotAlpha?: number;
  spotMode?: string; spotTint?: [number, number, number]; spotT?: number;
  videoMode?: string; videoAlpha?: number; videoT?: number;
  camRect?: [number, number, number, number]; camRadius?: number; dimCamera?: boolean;
}
```

Parameters for one FX-overlay render pass: click ripples (`style`/`color`/`intensity`/`hits`) plus the optional spotlight (`spot*`) and full-screen video-fx (`video*`) overlays, all in FX-render pixel space (`ow`/`oh`). The `cam*` fields are the webcam PiP's exclusion rect - mirrors the export's `Spot.cam_rect`/`cam_radius`/`dim_camera`, letting the backend undo the spotlight dim inside the webcam panel when `dimCamera` is false.

### Used by

- `src/lib/ipcPreview.ts` - parameter type of `previewFxOverlay`.
- `src/editor/stage/fxOverlay.ts` - `requestFxOverlay` builds this from the current click/spotlight/video-fx preview state before calling `previewFxOverlay`.

## previewFxOverlay

```ts
export const previewFxOverlay = (p: FxOverlayParams) =>
  invoke<string>("preview_fx_overlay", { /* p, with every optional field normalized to ?? null */ })
```

Render the FX overlay (spotlight + click effects) using the exact export shaders.

### Inputs

- `p: FxOverlayParams` - the render parameters. Every optional field is normalized to `?? null` before crossing the IPC boundary, since Tauri's `invoke` does not accept `undefined` in a serialized argument.

### Returns

`Promise<string>` - a PNG data URL of the overlay to composite on the preview canvas, rendered with the exact same GPU/CPU shader pipeline as the export.

### Used by

`requestFxOverlay` (`src/editor/stage/fxOverlay.ts`) - builds `FxOverlayParams` from the current preview state and calls this; in turn used by `src/editor/hooks/useCompositeLoop.ts`'s per-frame compositing.
