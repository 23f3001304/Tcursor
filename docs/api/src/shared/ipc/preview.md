# src/shared/ipc/preview.ts

The preview surface of the IPC client: the per-project tracks and layouts the editor stage composites from (`cameraTrack`, `previewLayout`, `previewLayouts`, `clickTrack`), the background art (`previewBg`, `backgroundThumbs`), the exact-frame render (`previewFrame`) and the backend FX overlay (`previewFxOverlay`), with the DTOs they return. Split back out of `ipc.ts` - this is the editor-preview vocabulary, while `ipc.ts` keeps recording, settings, the edit document and the project/asset commands. `ipc.ts` re-exports everything here, so `from "../shared/ipc"` keeps working everywhere.

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
export interface PreviewLayout { screen: [number, number, number, number]; radius: number; cam: [number, number, number, number, number, number, number, number, number] | null; canvas: [number, number]; screenAlpha?: number; camAlpha?: number; src?: [number, number, number, number] }
```

`src` is the canvas sub-rect the screen panel shows (fractions of the recorded canvas), the mirror of the export's `Scene.src`: the whole canvas (`[0, 0, 1, 1]`, and the default when absent) unless a mid-take display switch cropped it. `drawPreview` crops the screen video by it and `fxGeometry`'s `mapCanvas` maps canvas points through it, exactly as Rust's `coordmap::to_panel` does.

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

- `src/shared/ipc.ts` - field of `LayoutPresetDto`, and (as `| null`) of `SegRectDto`.
- `src/editor/timeline/model/layoutTrack.ts` - `lerpRect` cross-fades between two `PanelRectDto`s.

## LayoutPresetDto

```ts
export interface LayoutPresetDto { screen: PanelRectDto; cam: PanelRectDto; arrangement: Arrangement }
```

One layout preset's two panels: `screen` (the zoomed base layer) + `cam` (the fixed top layer), plus the same preset expressed as POSES.

- `arrangement: Arrangement` - this preset as a pose pair, ready to feed a `set_arrangement` op so a segment based on the preset becomes directly manipulable. *Why it comes from the backend rather than being computed here from `screen`/`cam`:* resolving it back must reproduce the preset's own pixels, and Rust owns that geometry (`scene::arrangement::arrangement_of_preset`, proven to 0.5px by its parity test). A hidden panel reports `null`, matching the `Arrangement` rule.

### Used by

- `src/shared/ipc.ts` - value type of `LayoutPresets`.
- `src/editor/timeline/model/layoutTrack.ts` - `presetOf` looks up the preset for a given `layout` name; `resolvedPanelsFor` falls back to it per panel when a segment has no `segs` override (it reads only `screen`/`cam`, never `arrangement`).

## LayoutPresetName

```ts
export type LayoutPresetName = "screen" | "camera" | "presenter" | "screen_only" | "camera_only";
```

The 5 layout preset names (matches `LayoutSeg.layout`'s known values).

### Used by

- `src/shared/ipc.ts` - type of the 5 named preset fields on `LayoutPresets`.
- `src/editor/timeline/model/layoutTrack.ts` - `KNOWN` validates a `LayoutSeg.layout` string against this set, falling back to `"screen"` for an unrecognized name.

## SegRectDto

```ts
export interface SegRectDto { id: string; screen: PanelRectDto | null; cam: PanelRectDto | null }
```

One `EditDoc.layout` segment's own resolved panels, by id (T34 L2). `null` on a field means "this segment doesn't override that panel - fall back to its `layout` preset", which is both fields on a segment with no `arrangement` at all; a segment WITH one always resolves both (never a mix), since Rust's `resolve_arrangement` always returns a full scene - a panel the arrangement hides still gets a real rect, just `alpha: 0`. Resolved in Rust through the exact function the export's `LayoutTrack` uses per segment (`scene::layout::resolve_seg_scene`) - the TS side never re-derives a pose, only picks which already-resolved rect to show.

### Used by

- `src/shared/ipc.ts` - element type of `LayoutPresets.segs`.
- `src/editor/timeline/model/layoutTrack.ts` - `resolvedPanelsFor` looks a segment's entry up by id, falling back to its preset per panel.

## LayoutPresets

```ts
export interface LayoutPresets {
  screen: LayoutPresetDto; camera: LayoutPresetDto; presenter: LayoutPresetDto;
  screen_only: LayoutPresetDto; camera_only: LayoutPresetDto;
  segs: SegRectDto[];
  spans: SourceSpanDto[];
  inset_w: number;
}
```

All 5 layout presets' panel rects, keyed by name, plus `segs` - one entry per `EditDoc.layout` segment (T34 L2), in doc order, not just posed ones. An explicit interface rather than `Record<LayoutPresetName, LayoutPresetDto>` (its pre-T34-L2 shape) because `segs` isn't a `LayoutPresetDto`; indexing by `LayoutPresetName` (`presetOf`'s `presets[name]`) still type-checks, since that type is a union of literal keys this interface actually declares.

`inset_w` (fraction of output width, mirrors `FrameRenderer::inset_w_frac`) is the export's fixed reference width for the synthetic cursor's scale-down (`cursorset::draw`'s `panel` factor) - a baseline independent of the active preset/arrangement, NOT one of the panel rects above. `cursorPanel.ts`'s `panelFactor` divides the active screen panel's own width by this so the preview cursor shrinks exactly like the export shrinks it for a narrow custom-arrangement panel.

### Used by

- `src/editor/hooks/doc/useEditorData.ts` - fetched via `previewLayouts` into state, passed down to `Stage`.
- `src/editor/timeline/model/layoutTrack.ts` - `layoutAt` cross-fades between presets AND per-segment overrides as the playhead crosses `LayoutSeg` boundaries.
- `src/editor/hooks/stage/useCompositeLoop.ts` - held in a ref so the per-frame compositing loop can resolve the current layout (and `inset_w`, for the cursor's panel scale/clip) without waiting on React state.

## SourceSpanDto

```ts
export interface SourceSpanDto { start_ms: number; src: [number, number, number, number]; transition_ms: number; fit: [number, number] }
```

One SOURCE SPAN of the take (mirrors Rust `export::render::spans::SourceSpan`): from `start_ms` on the output clock, the screen panel shows `src` - `[x, y, w, h]` as fractions of the recorded canvas - instead of the whole canvas. A mid-take display switch keeps ONE encoder canvas and fits every later frame into it, so the recording carries baked black bars from the switch on; the render shows only the active SOURCE SPAN's rect instead (`export::render::spans`).

`transition_ms` is the switch's ease and cross-dissolve length; `fit` is `[w, h]` ratios saying how much smaller that span's screen panel is than the full-canvas one, so the live preview can give a switched-to display its own aspect by scaling the resolved panel about its centre rather than re-deriving the export's panel math (`sourceSpans.ts`'s `fitPanel`).

A take that never switched display has exactly one span, at 0, covering `[0, 0, 1, 1]` with `fit` `[1, 1]`.

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

`useEditorData` (`src/editor/hooks/doc/useEditorData.ts`) - fetched on `[folder, rev]` (a layout edit, INCLUDING a `set_arrangement`/`clear_arrangement` op, is one of the things `rev` bumps for - see `Editor.tsx`'s `applyOp`, which bumps `rev` for every op except one ending in `_effect`).

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

## previewFrame

```ts
export const previewFrame = (folder: string, outMs: number) => invoke<string>("preview_frame", { folder, outMs })
```

The export's own composited frame at `outMs` (output time) as a JPEG data URL, from the warm Rust renderer (`export::preview::preview_frame`). Called by `useExactFrame` whenever the stage's playhead rests, so the frame being judged is a frame of the export - including, inside a clip dissolve, the outgoing clip blended under the incoming one.

**Output time, not clip time, since Batch 4 (clips).** A clip list can reorder the recording, and one source instant can then be shown twice; the backend's `out_of` answers with the first showing, so a playhead resting in the second one used to be handed the wrong frame. `useExactFrame` passes `tOut`, which the stage computes for every tick anyway, and the backend runs `clip_of` to find the source instant to decode. Inside a segment that round trip is the identity, so an unsplit project asks for exactly the frame it always did.

### Inputs

- `folder` (`string`) - project directory.
- `outMs` (`number`) - the instant, OUTPUT time, whole milliseconds.

## previewBg

```ts
export const previewBg = (folder: string) => invoke<string>("preview_bg", { folder })
```

### Inputs

- `folder` (`string`) - project directory.

### Returns

`Promise<string>` - a `data:image/png;base64,...` URL of the export background (mesh/gradient), so the canvas preview paints the exact same background the export uses. The frontend decodes it into an `<img>` the compositor draws under the screen.

## GradientStops

```ts
export interface GradientStops { from: [number, number, number]; mid: [number, number, number] | null; to: [number, number, number]; angle_deg: number }
```

A gradient tile's actual stops (two, or three when `mid` is set) and its angle. *Why the tile carries values and not just an id:* selecting a gradient preset writes `gradient_from`/`gradient_mid`/`gradient_to`/`gradient_angle_deg` straight into settings, so the panel needs the numbers. Shipping them on the tile keeps the Rust table (`settings::wallpapers::GRADIENT_WALLPAPERS`) the only copy - the TypeScript mirror that used to live in `backgroundPresets.ts` was removed with this change precisely because it could drift from the render.

## BackgroundThumb

```ts
export interface BackgroundThumb { id: string; name: string; kind: "mesh" | "gradient"; group: string; png_base64: string; gradient?: GradientStops }
```

One background-picker tile, rendered at 96x54 by the EXPORT's own background code, so a tile is a true miniature of what picking it produces.

- `id` - written to `background.mesh` for a wallpaper; selection state only for a gradient.
- `name` - the tile's label.
- `kind` - which `BackgroundKind` the tile applies.
- `group` - the picker section this tile belongs to: a wallpaper group (`Ribbons`, `Folds`, `Gradients`, `Metal`, `Scenic`, and whatever else ships later) or `Presets` for the procedural gradients.
- `png_base64` - the tile image, or EMPTY when the backend could not decode it (no ffmpeg). The grid then paints a plain swatch: the wallpaper is still selectable and still renders at export time, so hiding the tile would remove a working choice.
- `gradient` - present on gradient tiles only.

## backgroundThumbs

```ts
export const backgroundThumbs = () => invoke<BackgroundThumb[]>("background_thumbs")
```

### Inputs

None.

### Returns

`Promise<BackgroundThumb[]>` - every bundled wallpaper (grouped, in section order) then every gradient preset, in the order the panel shows them. Rendered once per process in Rust (a `OnceLock`), so calling this on every `BackgroundPanel` mount costs a clone after the first time.

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

- `src/shared/ipc.ts` - parameter type of `previewFxOverlay`.
- `src/editor/stage/fx/fxOverlay.ts` - `requestFxOverlay` builds this from the current click/spotlight/video-fx preview state before calling `previewFxOverlay`.

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

`requestFxOverlay` (`src/editor/stage/fx/fxOverlay.ts`) - builds `FxOverlayParams` from the current preview state and calls this; in turn used by `src/editor/hooks/stage/useCompositeLoop.ts`'s per-frame compositing.
