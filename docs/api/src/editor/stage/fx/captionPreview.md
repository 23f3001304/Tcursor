# src/editor/stage/fx/captionPreview.ts

The preview half of the caption LAYOUT: the TS mirror of Rust `export::fx::caption::captionlayout`, character for character. The blit that consumes it lives next door in `captionDraw.ts`, the same split the Rust side makes.

Parity here is over LAYOUT NUMBERS, not pixels (ADDED-5). `ab_glyph` and the browser's rasterizer cannot agree byte for byte, so both sides compute the caption's geometry and its animation state from character counts, timings and the same constants, and each then centres its OWN measured glyph run inside the resulting pill. `captionPreview.test.ts` and `captionlayout_tests.rs` pin the identical worked cases AND the identical 20-row `PARITY` animation fixture, so the two cannot drift apart without a test failing on both sides.

This is the one deliberate departure from the "export is the reference, route preview frames through Rust" rule: a backend PNG per frame would make captions lag by a full IPC round trip during playback, which is the one thing a caption must never do. While the playhead rests, `useExactFrame` still paints the export's real frame over the whole composite, so the resting image remains the export's own.

## ADV

```ts
export const ADV = 0.52;
```

Average glyph advance as a fraction of the font size - the shared constant, not a measurement. Mirrors Rust `captionlayout::ADV`.

## LINE_H

```ts
export const LINE_H = 1.32;
```

Line height as a multiple of the font size.

## PAD_X

```ts
export const PAD_X = 0.6;
```

Horizontal pill padding on each side, as a multiple of the font size.

## PAD_Y

```ts
export const PAD_Y = 0.34;
```

Vertical pill padding, top and bottom.

## MARGIN

```ts
export const MARGIN = 0.075;
```

Distance from the frame edge, as a fraction of output HEIGHT on both axes.

## RISE_LINES

```ts
export const RISE_LINES = 0.5;
```

How far below its resting place a `rise` caption starts, in line heights. Mirrors Rust `captionlayout::RISE_LINES`.

## POP_FROM

```ts
export const POP_FROM = 0.92;
```

The font scale a `pop` caption enters at. Mirrors Rust `captionlayout::POP_FROM`.

**Gone:** `FADE_MS`. The fade length is `style.animation_ms` now (default 120, so nothing changes for a project that never touched it), exactly as on the Rust side.

## MAX_CHARS

```ts
export const MAX_CHARS = 42;
```

A caption line's character budget.

## MAX_LINES

```ts
export const MAX_LINES = 2;
```

How many lines one caption may hold.

## LaidCaption

```ts
export interface LaidCaption {
  lines: string[]; fontPx: number; lineH: number;
  pill: [number, number, number, number]; baselines: number[];
  hi: number | null; alpha: number; rise: number; scale: number;
  wordsShown: number | null; wordAlpha: number;
}
```

One caption's geometry AND animation state at one instant, in CANVAS px. Mirrors Rust `LaidCaption` field for field; `hi` and `wordsShown` are `number | null` where Rust has `Option<usize>`, and `pill` is `[x, y, w, h]`. The pill is the text's box whether or not it is painted, and `rise` is already folded into `pill[1]` and every baseline (it is reported as well so a test can pin it).

## easeOut

```ts
export function easeOut(p: number): number
```

`1 - (1 - p)^3` on a clamped `p`. The ONE easing in the caption renderer, spelled exactly as Rust `captionlayout::ease_out` - if these two ever differ the caption drifts on screen without any test noticing, so they are pinned together in the parity fixture.

## ramp

```ts
export function ramp(dt: number, ms: number): number
```

A clamped linear `dt / ms`, and 1 when `ms <= 0` - which is what makes `animation_ms: 0` mean "no ramp" instead of `Infinity`. Mirrors Rust `captionlayout::ramp`.

## heightFrac

```ts
export function heightFrac(style: CaptionStyle): number
```

The line height as a fraction of frame height: `font_pct / 100` clamped to 0.015..0.08 when the fine size is set, else the `size` rung from `HEIGHT_FRAC` (`s: 0.03, m: 0.038, l: 0.048`). Mirrors Rust `CaptionStyle::height_frac`.

## wordSpan

```ts
export function wordSpan(cap: Caption, i: number): [number, number] | null
```

Word `i`'s half-open CHARACTER range inside the caption's joined text; `null` past the end. Mirrors Rust `captionlayout::word_span`, and serves both the highlight and the `words` reveal so the two can never disagree about where a word is.

One knowing divergence: Rust counts `chars()` (Unicode scalars) where this counts `String.length` (UTF-16 units). They differ only above the BMP, which is also true of the wrap budget this file has always used, so it is the pre-existing parity seam rather than a new one.

## wrapLines

```ts
export function wrapLines(text: string, max: number): string[]
```

Greedy word wrap at `max` characters, mirroring Rust `captionlayout::wrap_lines`.

### Inputs

- `text: string` - the caption text, already trimmed by the caller.
- `max: number` - the per-line character budget.

### Returns

The wrapped lines, never empty strings. A word longer than the budget takes a line of its own rather than being cut mid-word.

### Notes

Splits on `/\s+/` with empties filtered, which is `split_whitespace`'s behaviour on the Rust side: a run of spaces or a newline collapses to one break opportunity.

## captionAt

```ts
export function captionAt(caps: Caption[], tMs: number): Caption | null
```

The caption covering `tMs`, end EXCLUSIVE; `null` in a gap. `tMs` is OUTPUT-clock time (`tOut` in the composite tick), because the caption track - like every region list - is remapped onto that clock by `remapDoc`.

## layoutCaption

```ts
export function layoutCaption(cap: Caption, style: CaptionStyle, ow: number, oh: number, tMs: number): LaidCaption
```

Geometry for one caption at one instant. Pure: no canvas, no measurement, no DOM.

### Inputs

- `cap: Caption` - the caption. Its `words` drive `hi` only; the drawn text is `cap.text`.
- `style: CaptionStyle` - `doc.settings.captions`: position, size or `font_pct`, highlight on/off, `animation` and `animation_ms`.
- `ow`, `oh` - the canvas backing-store size (the resolved aspect), which is the preview's stand-in for the export's output frame.
- `tMs` - the instant, on the output clock.

### Returns

A `LaidCaption`. An empty caption returns no lines, a zero pill and no baselines - but its `alpha`, `rise`, `scale` and reveal are still filled in, as on the Rust side.

### Implementation

Identical arithmetic to Rust `layout`, in the same order: the entry ramp, then `alpha` (0 outside the span for every kind, 1 for `none`, else the symmetric ramp), then the `pop` scale, the font size (floored at 8, then scaled), the `rise` offset, the LAST word that has started as `hi` (so a pause holds the previous word rather than blinking the highlight off), the `words` reveal through the private `reveal`, the wrap truncated to two lines, then pill width from the widest line's character count, pill height from the line count, centred x, `rise` added to y, and a baseline per line.

### Behaviors

`captionPreview.test.ts` pins parity case A (bottom, medium, one line, 1920x1080) and case B (top, small, two lines, mid fade, 1280x720) against the exact numbers `captionlayout_tests.rs` asserts - both UNCHANGED by the M5 look pass, which is how "the defaults render exactly as before" is enforced rather than claimed. It then pins the 20-row `PARITY` table (five animation kinds x entry midpoint / steady / exit midpoint / past the end, each with `alpha`, `rise`, `scale`, `wordsShown`, `wordAlpha` and the `fontPx` / pill `y` they imply), the rise moving pill and baselines together, the pop re-layout, the `words` fallback with no timings, `animation_ms: 0`, the clamped `font_pct`, `wordSpan` and `easeOut` - alongside the fade, the exclusive end, the empty caption, the highlight hold and the four wrap cases.

### Private helpers

#### reveal

`function reveal(cap, style, tMs): [number | null, number]` - the `words` animation's `[wordsShown, wordAlpha]`, or `[null, 1]` for every other kind and for a `words` caption with no word timings (the fallback to `fade`). Mirrors Rust `captionlayout::reveal`.

### Used by

- `src/editor/stage/fx/captionDraw.ts` - `drawCaptions`, the only caller in the preview.
- `src/editor/panels/captions/CaptionList.tsx` - `captionAt`, to mark the caption under the playhead.
