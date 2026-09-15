# src/editor/panels/captions/captionLook.ts

The vocabulary the Captions panel's Look block is built from: the size rungs and the two slider ranges, the animation menu with its one-line explanations, and the three colour palettes. Data only, so the rung/slider agreement can be asserted without a renderer and so `CaptionStyleControls.tsx` and `CaptionColorFields.tsx` cannot drift apart on what "Medium" means.

## SIZE_RUNG_PCT

```ts
export const SIZE_RUNG_PCT: Record<CaptionSize, number>
```

`{ s: 3.0, m: 3.8, l: 4.8 }` - the TS mirror of Rust `CaptionSize::height_frac` (`settings/captions.rs`), in percent rather than in fraction because that is the unit the slider and its readout speak.

## LINE_PCT_MIN

```ts
export const LINE_PCT_MIN = 2.0;
```

The bottom of the Line height slider, in percent of frame height. With `LINE_PCT_MAX` it is narrower than Rust's own 1.5% to 8% clamp on purpose: the clamp is what keeps a hand-written `edit.json` from rendering a caption taller than the frame, while this is the range a person would actually want to drag through.

## LINE_PCT_MAX

```ts
export const LINE_PCT_MAX = 6.0;
```

The top of the Line height slider. All three size rungs sit inside `LINE_PCT_MIN`..`LINE_PCT_MAX`, so picking one never pins the thumb against an end.

## LINE_PCT_STEP

```ts
export const LINE_PCT_STEP = 0.1;
```

Tenths of a percent - one step is about a pixel of cap height on a 1080p frame, which is the finest difference worth offering and coarse enough that `captionRung` can match a rung exactly.

## ANIM_MS_MAX

```ts
export const ANIM_MS_MAX = 600;
```

The top of the Duration slider. Long enough to read as deliberate, short enough that a caption cannot still be arriving when the next one starts.

## ANIM_MS_STEP

```ts
export const ANIM_MS_STEP = 20;
```

20 ms - finer than anyone can see, and coarse enough that the readout never shows a number nobody chose.

## captionLinePct

```ts
export const captionLinePct = (s: CaptionStyle) => number
```

The line height the panel SHOWS, in percent of frame height: `font_pct` when it is set, else the `size` rung's own value. `font_pct: 0` means "use the rung" throughout the model (Rust reads it the same way), so a project that never touched the slider still puts the thumb somewhere honest instead of at zero.

### Behaviors

- `reads a project that never touched the slider off its size rung` - `font_pct: 0` with each of the three rungs.

## captionRung

```ts
export function captionRung(s: CaptionStyle): CaptionSize | ""
```

Which of Small / Medium / Large should light: the rung whose value the current line height is exactly on, or `""` for none. `""` rather than `null` because `Segmented` takes a value of its own option type and lights nothing when no option matches - which is precisely the state a dragged slider should leave the rungs in.

Comparison is to within 0.001, not `===`: the slider's own `snapToStep` rounds to the step's decimal places, so 3.8 comes back as 3.8, but the tolerance costs nothing and means no float ever leaves a rung dark that should be lit.

### Behaviors

- `lights the rung the fine size sits exactly on, and no rung between two` - the default lights Medium, `font_pct: 4.8` lights Large whatever `size` says, `4.2` lights none.
- `puts every rung inside the slider's own range, so picking one never pins the thumb`.
- `keeps the slider showing what the rung wrote, so the two never disagree`.

## linePctText

```ts
export const linePctText = (v: number) => string
```

The slider's readout: `"3.8% of height"`. Says what the number is a percent OF, because a bare "3.8%" beside a caption invites reading it as an opacity.

## ANIMATIONS

```ts
export const ANIMATIONS: { value: CaptionAnim; label: string; title: string }[]
```

The Animation picker's menu, in order: None, Fade, Rise, Pop, Word by word. Each carries a one-line `title` the `Picker` shows on hover, phrased as what the viewer sees rather than as what the renderer does - "Each word appears as you say it", not "reveals at each word's start time". Word by word names its own fallback, because a caption with no word timings is a real state (a hand-typed one, or a split) and a silently different animation would read as a bug.

The labels and their meanings mirror Rust `CaptionAnim` (`settings/captions.rs`); the renderer is the authority on what each one draws.

All three palettes below are `NamedColor` (`panels/effectSwatches.ts`), so they go through the same `swatchItems` the click effects and spotlight tints already use rather than a second swatch idiom, and each is a starting point rather than the whole choice - every one of the three fields carries a `ColorInput` beside it for anything else.

## TEXT_COLORS

```ts
export const TEXT_COLORS: NamedColor[]
```

White, warm white, yellow, cyan - the four that stay legible over arbitrary video, warm white and yellow being what broadcast captions have used for decades.

## HIGHLIGHT_COLORS

```ts
export const HIGHLIGHT_COLORS: NamedColor[]
```

Yellow, green, blue, red. The list does NOT include the interface accent: that is the first swatch in the panel, and it is `null`, not a colour (see `CaptionColorFields.md`).

## PILL_COLORS

```ts
export const PILL_COLORS: NamedColor[]
```

Black, deep navy, white. Three, not five: a pill is a scrim, and its job is to be ignored.

### Used by

- `src/editor/panels/captions/CaptionStyleControls.tsx`
- `src/editor/panels/captions/CaptionColorFields.tsx`
