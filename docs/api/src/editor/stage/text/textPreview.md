# src/editor/stage/text/textPreview.ts

The preview's half of the text layout: the same contract as `src-tauri/src/export/fx/text/textlayout.rs`, transcribed, and pinned to it by the same generated table.

**This file is a mirror, not an independent implementation.** Every constant, every branch and every ordering in `layOne` corresponds one for one to `lay_one`. The 48 rows in `textPreview.test.ts` are the same 48 rows `textlayout_tests.rs` holds, copied from the Rust file byte for byte rather than regenerated here, and they were produced by `cargo test --lib print_text_parity_table -- --ignored --nocapture`. The export is the reference (owner ruling 2026-09-14): when the two disagree, the fix goes here.

What makes the mirror possible at all is that the block is measured from CHARACTER COUNTS at the fixed advance `ADV`, never from font metrics. A browser cannot load the export's `ab_glyph` face, so if the box depended on real Inter advances the two sides could never agree. Instead the shared contract decides the BOX from counts, and each side centres its own truly measured run inside it (`textDraw.ts::runX` with `ctx.measureText`, `textdraw.rs::run_x` with `glyph::run_width`). That split is the ADDED-5 idiom, first made for captions.

`ease` comes from `src/editor/timeline/model/layoutTrack.ts`, the M3 evaluator with the `Keys` arm. Its Rust twin is `crate::export::camera::ease`, NOT `crate::export::easing::ease`: the two Rust functions disagree on `smooth`, and only the camera one returns `t * t * (3 - 2 * t)` the way this evaluator does.

## ADV

```ts
export const ADV = 0.52;
```

The nominal character advance as a fraction of the font size. The Rust side re-exports it from `captionlayout`, which owns it because captions landed first (spec 4 fit point 3); here it is a literal, so the two copies of the number are what the pinned table guards.

## LINE_H

```ts
export const LINE_H = 1.32;
```

The line height as a multiple of the font size. Captions' constant, same arrangement as `ADV`.

## SUB_RATIO

```ts
export const SUB_RATIO = 0.58;
```

The second line's size as a fraction of the main line's.

## PAD_X

```ts
export const PAD_X = 0.6;
```

The plate's horizontal padding on each side, as a fraction of the main font size.

## PAD_Y

```ts
export const PAD_Y = 0.34;
```

The plate's vertical padding above and below, as a fraction of the main font size. Smaller than `PAD_X` because `LINE_H` already carries leading inside the text rows.

## MARGIN

```ts
export const MARGIN = 0.06;
```

The safe-area inset from the frame edge for an edge anchor, as a fraction of output HEIGHT. Height for both axes deliberately, so the inset is the same optical size on a portrait and a landscape export.

## RULE_W

```ts
export const RULE_W = 0.006;
```

The `bar` style's rule width as a fraction of output height. The block reserves twice this: one width for the rule, one for the gap.

## SLIDE_FRAC

```ts
export const SLIDE_FRAC = 0.04;
```

How far a `slide` travels at `g = 0`, as a fraction of output height.

## POP_FROM

```ts
export const POP_FROM = 0.86;
```

The scale a `pop` starts from, growing to 1 so it never overshoots.

## LaidText

```ts
export interface LaidText { main, sub, fontPx, subPx, plate, mainBaseline, subBaseline, rule,
  alpha, reveal, scale, shift, fill, shadow, plateRgb, plateAlpha, ruleRgb, centred, boxW }
```

The camelCase twin of the Rust `LaidText`, with two differences worth stating:

- **`reveal: number | null`** where the Rust is `usize`. JavaScript has no `usize::MAX`, so `null` means "draw the whole line". The pinned table encodes it as `-1` on both sides, which is why `flat()` maps it.
- **`boxW: number`** is TS-only and is deliberately NOT in the pinned table. It is the text box width the layout produced (`textW`), which the Rust `run_x` recomputes from character counts when it needs it. Carrying it is simpler in TypeScript and changes no pinned number, because the table's twenty fields are geometry and timing only.

`fill` and `ruleRgb` are separate resolved colours and that is the `bar` style's whole point: spec 5.4's table gives `bar` white glyphs and a `ui.accent` rule, so the two cannot be one field.

## layOne

```ts
export function layOne(item: TextItem, accent: [number, number, number],
                       ow: number, oh: number, tMs: number): LaidText | null
```

Lay one text item out at one instant on the output clock, or `null` if it should not be drawn.

**It implements the identical contract to `export::fx::text::textlayout::lay_one`**, whose docs page carries the full statement of it. The five points that page rules, restated in one line each so a reader here knows what to keep in step:

1. **Nothing to draw** - `null` outside the half-open span `[start_ms, end_ms)`, and `null` when the trimmed main AND the trimmed sub are both empty (nothing at all, not a bare plate).
2. **The two ramps** - `p` and `q` are the clamped in and out progresses, `g = min(ease(p), ease(q))`, and **the governing animation is `anim_in` when `eIn <= eOut` and `anim_out` otherwise**, which is whichever ramp produced the minimum. `min` is what lets an in plus an out longer than the span simply never reach full instead of snapping.
3. **What each animation does** - fade takes `alpha = g`; slide adds `shift = (1 - g) * oh * SLIDE_FRAC * dir`, where `dir` points at the anchor's own edge and is zero at the centre; pop scales from `POP_FROM`; typewriter is at full strength from its first frame and only fades at the very end. `reveal` keys on **`anim_in`**, not on the governing animation, so the out-ramp cannot un-type a finished line.
4. **`scale` is applied to `fontPx` before the block is measured**, so a centre anchor grows about its centre and an edge anchor pins its edge. Nothing downstream re-applies it.
5. **The rule is on the left** for a left or centre anchor and on the right for a right anchor, and reserves `2 * oh * RULE_W` of block width so the run clears it by one rule width.

### Inputs

- `item` - the document's item, already on the OUTPUT clock (`remapDoc`), so `tMs` and `item.start_ms` share a time base.
- `accent` - `Settings.ui.accent`, RGB. One accent per document, read at draw time and never stored on the item (ADDED-4).
- `ow`, `oh` - the CANVAS size, which stands in for the output frame; every length is a fraction of one of them.
- `tMs` - the instant, output clock.

### Returns

`LaidText | null`.

### Behaviors

- `pins the same forty eight layouts` - the parity table, compared field by field to three decimal places.
- `returns null outside the span and for an empty item`.
- `reveals a typewriter on the same fixed cadence` - 0, 10, 20, 40, 40 characters, the same five instants the Rust test uses.
- `keeps array order and drops the ones that are not live`.

## laidTexts

```ts
export function laidTexts(items: TextItem[], accent: [number, number, number],
                          w: number, h: number, tMs: number): LaidText[]
```

`layOne` over an array, keeping the live ones in ARRAY ORDER, which is creation order. There is deliberately no `layer` field on a text item (spec 5.1): the nine anchors place items where the user put them, and an overlap is a mistake the user can see and fix rather than a priority contest the way competing zooms were. The timeline lane stacks overlapping items onto rows for legibility only; those rows never affect paint order.

### Used by

- `src/editor/stage/fx/overlayDraw.ts` - once per tick, feeding `drawTexts`.
