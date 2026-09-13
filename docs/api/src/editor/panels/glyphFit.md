# src/editor/panels/glyphFit.ts

Two pure functions for fitting a cursor sprite into a tile. Added by the panel pass after the owner's read of the pack grid: "pack tiles look different in colour and padding from one another".

**The measured cause.** Every pack ships 128x128 PNGs, but the drawing inside them is padded differently per pack - content widths run about 70 to 105px. Drawing the files at one fixed box therefore makes the same arrow look large in one tile and small in the next, which reads as sloppy rather than as a difference between packs.

**What is deliberately NOT done here.** Nothing tints, recolours, inverts or shadows a glyph. The packs are coloured on disk (Cartoon orange, Gradient Glass blue, Neon purple, Outline navy, Cat cream, Ink black) and a tile must show the colour the pack actually draws (owner ruling, 2026-09-13). Legibility for the dark packs comes from the constant mid-grey plate behind the glyph (`.e-glyph-plate`, `panels.css`), not from any filter on the glyph.

## Box

```ts
export interface Box { x: number; y: number; w: number; h: number }
```

A rectangle in image pixels.

## alphaBounds

```ts
export function alphaBounds(data: Uint8ClampedArray, w: number, h: number, threshold = 8): Box | null
```

The tight bounds of everything at least `threshold` opaque in an RGBA buffer, or `null` when the image is fully transparent (nothing to fit - the caller then keeps its unfitted fallback).

`threshold` defaults to 8 rather than 1 because an anti-aliased PNG's edges are full of alpha-1-to-5 pixels, and a single stray one would stretch the box by the whole padding it was meant to trim.

### Behaviors

- `finds the drawn content inside a transparently padded sprite`.
- `returns null for a fully transparent image, so the caller keeps its fallback`.
- `ignores all-but-invisible pixels, which anti-aliased PNG edges are full of`.

## fitBox

```ts
export function fitBox(box: Box, imgW: number, imgH: number,
  fitW: number, fitH: number, plateW: number, plateH: number): { width: number; height: number; left: number; top: number }
```

Where to put the WHOLE image so that `box` - its drawn content - fills the `fitW` x `fitH` area as far as it can without distortion, centred in a `plateW` x `plateH` plate. The result is CSS pixels for an absolutely positioned `<img>`.

One uniform scale (`min` of the two axis ratios), so the image is never squashed; the offsets place the CONTENT box's centre on the plate's centre, not the image's, which is the whole point - a sprite drawn in the top-left corner of its canvas still lands centred.

### Behaviors

- `scales by the content box, so differently padded packs match`.
- `fits the limiting axis and never distorts`.
- `centres the content box in the plate, not the image`.

### Used by

- `src/editor/panels/PackTile.tsx` - `useGlyphFit` measures once per sprite on an offscreen canvas and feeds the result to `GlyphPlate`'s `<img>` style.
