# src/editor/stage/cursor/cursorGlass.ts

The glass cursor material, as much of it as a 2D canvas can do. Split out of `cursorPreview.ts` for its size budget; the export's own version is Rust `export/fx/lens/mod.rs` plus `fx_lens.wgsl`, and every number here is that file's.

**DELIBERATE DIFFERENCES from the export**, all of them "less, never elsewhere":

- The back is a plain magnifier: the canvas re-drawn onto itself `LENS_ZOOM` bigger about the shape's centre (`drawImage` snapshots its source first, so the canvas IS the undisturbed frame the export re-samples) - the readable part of the glass, and the part the owner asked for. No rim bend, no frost, no rim light.
- A glass pack's sprite-shaped lens does not magnify at all: a 2D canvas cannot clip to a sprite's alpha in one pass, and the sprite is small enough that the missing zoom is not what the eye reads.
- No drop shadow, no click squash, no ink drop or over-text ring.
- The back does not stretch along a text selection.

What DOES match is every position, size and shape, including the state cross-fade. That is the point: scrubbing to a pause swaps in the backend's exact frame (`useExactFrame`), and nothing may MOVE when it does - the glass simply resolves.

## GLASS_ALPHA

```ts
export const GLASS_ALPHA = 0.65;
```

The alpha a glass pack's sprite is drawn at, mirroring Rust `fx_lens::SPRITE_ALPHA`. Pinned by `cursorPreview.test.ts`.

## LENS_ZOOM

```ts
export const LENS_ZOOM = 1.35;
```

The magnification inside a glass shape - Rust `fx_lens::ZOOM` / `fx_lens.wgsl::LENS_ZOOM`. `drawBack` uses it as the ratio between its source rect and the shape it fills; the numbers on the three sides must stay equal or a paused frame (the export's own) would zoom differently from the live canvas around it.

## BACK_SCALE

```ts
export const BACK_SCALE = 2.2;
```

The back's diameter as a multiple of the sprite's drawn height - Rust `fx_lens::BACK_SCALE`.

## PILL_W

```ts
export const PILL_W = 0.35;
```

The back's height over text as a fraction of its width - Rust `fx_lens::PILL_W`.

## MORPH_MS

```ts
export const MORPH_MS = 160;
```

How long a cursor state change eases over, in ms - Rust `fx_lens::MORPH_MS`.

## morphEase

```ts
export function morphEase(p: number): number
```

The click effects' ease-out cubic, which the morph shares - Rust `clickfx::ease_out`, also mirrored by `ripplePreview.ts::easeOut` and `fx_clicks.wgsl::fx_ease`. Pinned at the same five points all four sides are.

## cursorMorphAt

```ts
export function cursorMorphAt(kinds: CursorKindSample[], ms: number): { kind: string; prev: string; m: number }
```

One cursor state change in flight at output time `ms`: the kind now in force, the one before it, and how far the change has eased through `MORPH_MS` (1 = settled). The TS mirror of Rust `fx_lens::kind_morph`, down to "an empty track is a settled arrow, never a morph from nothing".

Same binary search as `cursorPreview.ts::cursorAt`, which it supersedes for glass packs - `cursorAt` remains for callers that only want the kind.

## spriteBox

```ts
export function spriteBox(img: HTMLImageElement, hot: [number, number], canvasH: number,
  p: [number, number], sizePx: number): [number, number, number, number]
```

A sprite's placed box in canvas px as `[x0, y0, w, h]` - the hotspot lands on `p`. The mirror of Rust `cursormorph::sprite_box`, including the canvas-relative scale that keeps a wide sprite wide.

## lerpBox

```ts
export function lerpBox(a, b, m): [number, number, number, number]
```

`a` lerped to `b` by an already-eased `m`, clamped - Rust `cursormorph::lerp_box`. Because both boxes put their own hotspot on the same point, every point along the interpolation does too; `cursorPreview.test.ts` pins that.

## backBox

```ts
export function backBox(kind: string, spriteH: number): { rx: number; ry: number }
```

The cursor back's half-extents for one cursor kind, given the sprite's DRAWN height in canvas px: a disc for every shape but the I-beam, which is a HORIZONTAL pill `PILL_W` as tall (a line of text is horizontal, and so is the selection it would stretch along). Rust `fx_lens::back_of`.

The corner radius is always the shorter half-extent, which is what makes one rounded rect serve as both circle and pill.

## drawBack

```ts
export function drawBack(ctx: CanvasRenderingContext2D, centre: [number, number],
  kind: string, prev: string, m: number, spriteH: number)
```

The cursor back for the live canvas: a magnifying rounded rect, morphing between kinds on the same eased progress the export uses. The shape is clipped, the canvas is drawn onto itself with the shape's box shrunk by `1/LENS_ZOOM` about the centre as the source rect and the whole shape as the destination (a uniform `LENS_ZOOM` zoom - Rust `fx_lens::ZOOM`), then a faint cool wash (`rgba(235,243,255,0.10)` - the nearest a fill gets to the shader's multiplicative lift without dimming the letters the magnifier just made readable) and the rim stroke go over it. Drawn before the trail and the sprite, so they land on top of the glass exactly as the export blits them over its refraction.

`centre` is the sprite's BOX centre, not the cursor point: an arrow's hotspot is its tip, so a disc centred there would sit up and left of the cursor it is behind. Matches Rust `fx_lensbuild::lenses_at`, which passes the same centre to `back_geom`.

The rim stroke stands in for the shader's `BACK_RIM` without its light direction - a flat stroke is what a fill-only disc is missing to read as an object rather than a smudge.

### Used by

- `src/editor/stage/cursor/cursorPreview.ts` - `drawCursorSprite`, before the trail and the sprite.
