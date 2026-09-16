# src/editor/stage/text/textDraw.ts

The 2D-canvas painter for the laid-out text items: the twin of `src-tauri/src/export/fx/text/textdraw.rs`.

It knows nothing about time, anchors or animation. Everything it needs has already been decided by `textPreview.ts::laidTexts`, which is the same split `textdraw.rs` has against `textlayout.rs`, and which is what keeps the two painters from being able to disagree about anything except their own text stacks.

## drawTexts

```ts
export function drawTexts(ctx: CanvasRenderingContext2D, laid: LaidText[]): void
```

Paint every laid item onto the stage canvas, in array order.

### Inputs

- `ctx` - the stage canvas context, mid-tick. Each item is wrapped in its own `save`/`restore`, so nothing here leaks into the captions drawn after it.
- `laid` - `laidTexts(...)`'s output for this tick. An item whose `alpha` has reached zero is skipped.

### Returns

`void`.

### The paint order, per item

1. **The plate** - `roundRect` (`../canvas/previewDraw.ts`) at `plateAlpha * alpha`, radius `fontPx * PLATE_RADIUS`, filled in `plateRgb`.
2. **The rule** - a plain `fillRect` in `ruleRgb`, the document accent, at the item's full alpha. Square-cornered on purpose: it is a bar, not a pill.
3. **The main run** - the revealed characters only (`reveal === null` means all of them), at `alpha`.
4. **The sub run** - at `alpha * SUB_ALPHA`.

The runs go last so the glyphs sit over their own background rather than being dimmed by it.

### The face

`600 <px>px "Inter Variable", system-ui, sans-serif` - the same family and weight as the export's embedded `Inter-SemiBold.ttf`, with a real fallback stack. The two are not required to be metrically identical, and the design does not assume they are: see `runX` below.

### The shadow

`shadowColor` at `rgba(0, 0, 0, 0.6 * alpha)` with a one-pixel offset down and right, matching `glyph::draw_run`'s optional shadow exactly, including the 0.6 and the scaling by the run's own alpha so it fades with the text. Only the two styles that sit directly on the picture ask for it.

## PLATE_RADIUS

```ts
const PLATE_RADIUS = 0.35;
```

The plate's corner radius as a fraction of the MAIN font size, which `roundRect` then clamps to half the box's width and height. A fraction of the type rather than of the plate, so a one-line and a two-line plate read as the same component. Same number as `textdraw.rs`'s.

## SUB_ALPHA

```ts
const SUB_ALPHA = 0.82;
```

The second line's opacity as a fraction of the item's own. It is what makes a lower third read as a name with a role under it rather than as two lines of equal importance. `textdraw.rs` applies the same 0.82; it is a rendering decision rather than a measurement, which is why it lives with the painters and not with the layout constants.

## PAD_X

```ts
const PAD_X = 0.6;
```

A local copy of the layout's plate padding, used by `runX` to recover the plate's INNER width from its outer one. It is duplicated rather than imported because `textDraw.ts` is otherwise free of layout knowledge and the parity table pins the value on the layout side anyway.

## runX

```ts
function runX(ctx: CanvasRenderingContext2D, l: LaidText, text: string, px: number): number
```

Where a centred run actually starts, measured in the browser's own font.

This is the ADDED-5 idiom and it is the reason the two sides do not need matching font metrics. The shared layout produced a BOX from character counts at the fixed advance `ADV`; inside that box each side centres its OWN measured run - here with `ctx.measureText`, in the export with `glyph::run_width` on real Inter advances. So both sides agree exactly on where the box is, each is exactly right about its own glyphs, and a fallback font on the viewer's machine shifts nothing but the run's own centring inside a box that is still in the pinned position.

The inner width is the plate's width minus its padding when the style plates, and `l.boxW` otherwise, which is the TS-only convenience field `layOne` carries for exactly this.

A non-centred item returns `l.mainBaseline[0]` unchanged and never measures anything, which is what a left or right anchor means.
