# src/editor/stage/mask/maskPreview.ts

The preview's mirror of `export::fx::mask::fx_masks`: the same projection from a mask's canvas-fraction rect to the output rectangle one frame draws, in TypeScript, so the editor stage shows what the export will write.

**The twenty-four row table is the SAME table.** `maskPreview.test.ts` pins the numbers `fx_masks_table_tests.rs` pins, generated once on the Rust side by `cargo test --lib print_mask_parity_table -- --ignored --nocapture` and retyped here. They are not regenerated from this file, and neither side's rows are hand-edited: a divergence between the two projections is then a failing test rather than a look bug nobody notices.

**Why the backend FX overlay cannot carry a mask** (spec 2.4). `export/preview/preview_fx.rs` recovers straight alpha from two renders, one over black and one over white: `a = 1 - (white - black) / 255`. A blur of solid black is solid black and a blur of solid white is solid white, so the difference is 255 everywhere and the recovered alpha is 0. A mask is not expressible as an over-composite, which is why the stage paints its own rather than asking the backend, and why `preview_fx.rs` is untouched by this feature.

**The three stated approximations.** The preview is a mirror, not a re-implementation, and it differs from the export in three named and bounded ways:

1. **The blur kernel.** The export runs three box passes (`maskdraw::blur_sigma`) or a 13-tap GPU disc; the preview uses `ctx.filter = blur(Npx)`, a CSS Gaussian. `blurSigmaFor` converts between them, which is what makes a 24 px export blur and a 34.6 px CSS blur the same picture.
2. **The feather.** A 2D canvas clip has a hard edge and `ctx.filter` on a clip path is not portable, so `maskDraw.ts` does not reproduce the soft edge at all. See `maskDraw.md`.
3. **The corner rounding.** The export rounds a rect's corners to whole source and panel pixels; the preview stays in floats; the two agree within one output pixel, which is under the softest feather the inspector allows.

On the third: `fx_masks::project_rect` quantises twice - the canvas fraction to an integer `FramePoint`, then `coordmap::to_panel` to an integer panel point - because that is the mandated chain the cursor and every click hit already take. `fxFrameGeometry` is the shared float geometry the rest of the preview FX uses and keeps no integers. The residual over the whole pinned table is at most 0.97 px, so the parity assertion for `mn`, `mx` and `r` is `<= 1.0` px while `featherPx`, which never passes through the projection, stays pinned at 1e-3.

**A note on the camera in that table.** The Rust `Camera` centre is in OUTPUT PIXELS and the preview's `cam.cx`/`cy` are PANEL FRACTIONS (`export/preview/preview_track.rs` builds the preview track as `clamp((cam.cx - panel.x) / panel.w, 0, 1)`, which is exactly what `fxFrameGeometry` inverts), so the test converts each Rust pose PER LAYOUT rather than once: panel-fraction 0.5 is output pixel 960 on the full-frame Screen layout but 768 on Presenter, whose panel starts at x 96 and is 1344 wide.

## DEFAULT_BLUR

```ts
export const DEFAULT_BLUR = 0.02;
```

The blur radius a Blur mask takes when it has no `strength` of its own, as a fraction of the output height. Mirrors `edit::effect::DEFAULT_BLUR`.

## DEFAULT_PIXEL

```ts
export const DEFAULT_PIXEL = 0.018;
```

The cell size a Pixelate mask takes with no `strength`, as a fraction of the output height. Mirrors `edit::effect::DEFAULT_PIXEL`. *Why it is smaller than the blur default:* a pixel cell reads as coarser than a blur of the same radius, so matching the numbers would make pixelate look much stronger than blur at the same slider position.

## DEFAULT_MASK_ROUNDNESS

```ts
export const DEFAULT_MASK_ROUNDNESS = 0.06;
```

The corner radius with no `roundness`, as a fraction of the projected rect's SHORT side. Mirrors `edit::effect::DEFAULT_MASK_ROUNDNESS`.

## DEFAULT_MASK_FEATHER

```ts
export const DEFAULT_MASK_FEATHER = 0.01;
```

The soft-edge width with no `feather`, as a fraction of the output height. Mirrors `edit::effect::DEFAULT_MASK_FEATHER`. The preview does not draw the feather (approximation 2 above), but it still carries the number so the value the inspector shows is the value the export uses.

## HIDDEN_PANEL_ALPHA

```ts
export const HIDDEN_PANEL_ALPHA = 0.5;
```

The screen-panel alpha below which no mask on that panel is drawn, because the panel itself is not. Mirrors `fx_masks::HIDDEN_PANEL_ALPHA`; see that page for why it is a threshold at the middle of the cross-fade rather than a test against zero.

## MaskKindId

```ts
export type MaskKindId = 1 | 2 | 3;
```

The numeric kind a mask travels as: 1 Blur, 2 Pixelate, 3 Highlight, mirroring `fx_masks::mask_kind_id`. 0, the empty-slot marker on the GPU, has no place here because the preview holds a list rather than fixed slots.

## MaskPx

```ts
export interface MaskPx {
  mn: [number, number]; mx: [number, number]; r: number; featherPx: number;
  amountPx: number; dim: number; kind: MaskKindId; alpha: number;
}
```

One mask as this frame draws it, every length in CANVAS PIXELS (the visible canvas, not the half-resolution FX buffer). Field for field the mirror of `fx_masks::MaskDraw`; see `fx_masks.md` for each one's meaning.

## blurSigmaFor

```ts
export function blurSigmaFor(amountPx: number): number
```

The Gaussian sigma that three box passes of radius `round(amountPx)` approximate: `sqrt(((2r + 1)^2 - 1) / 4)`, with `r` floored at one pixel. The exact twin of `maskdraw::blur_sigma`, pinned on both sides at the same three sample points, and the number `maskDraw.ts` hands to `ctx.filter`.

## maskDraws

```ts
export function maskDraws(effects: EffectRegion[], layout: PreviewLayout | null,
  cam: { cx: number; cy: number; scale: number }, w: number, h: number,
  tMs: number, defaultDim: number): MaskPx[]
```

Every mask live at `tMs`, projected and sorted by `(layer, index)` - the mirror of `masks_at`.

The projection goes through `fxFrameGeometry(w, h, layout, cam, 1)`, whose `mapCanvas` runs `toPanelFrac` and then the panel and camera mapping: the same two steps as `to_panel` then `project`, in floats. **The `1` is the FX scale**, passed deliberately so the result is in visible canvas pixels rather than the half-resolution buffer the FX overlay uses.

All FOUR corners are mapped and their bounding box taken, matching the Rust side; the camera never rotates, so that box IS the projected rectangle. The result is intersected with the panel's own projected rect and dropped if under a pixel in either axis, which is what makes a mask placed on another display draw nothing.

Like `masks_at`, alpha is strictly per region (its own fade ramp) and never consults a winner-take-all simulator: two blurs over two different passwords must both be drawn.

### Used by

- `src/editor/hooks/stage/compositeFrame.ts` (`drawCompositeFrame`) - once per frame, in the seam between the panels and the cursor.
- `src/editor/stage/mask/useMaskDrag.ts` (`useMaskDrag`) - to place the drag box over the selected mask.
