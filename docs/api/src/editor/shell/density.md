# src/editor/shell/density.ts

The editor's laptop-density model, as one pure function. The editor was drawn on a 4K monitor at
175% scaling (a 2194 x 1234 logical desktop); on a laptop - 1366 x 768, 1440 x 900 or 1536 x 864
logical, at 125-150% - the same fixed pixels read as oversized, the rail, the panel column and the
inspector eat the stage, and the timeline's filmstrip and lanes take height the preview needs. This
file decides, for a given viewport, how much smaller everything gets.

### The model, and why this one

**One unitless multiplier, `--e-density`, applied to the token rungs the editor already has.** Every
`--e-text-*` size, every `--e-sp-*` step, every radius and the five `--e-ctl-*` control heights in
`editor.css` is a `calc(<base>px * var(--e-density))`, so they step down together instead of one
sheet at a time. `Editor.tsx` writes the multiplier - with the stepped sizes below it - as an inline
custom property on the `.editor` element; the values declared in `editor.css` are the full-size
defaults, so the stylesheet is still correct with no JavaScript at all (a cold first paint, a test
that mounts a panel by itself).

Two alternatives were rejected:

- **A root font-size in `rem`.** The editor shares its document with the HUD, whose own tokens live
  on `:root`; moving the root font-size would reach a surface this pass does not own, and the
  editor's scale is not the root's font-size anyway - it is the token block on `.editor`.
- **Media queries alone.** `ClassicShell` springs the panel column and the properties sidebar to a
  width *in TypeScript*, so a CSS-only breakpoint would leave Motion animating to 320 while the
  stylesheet said 288; and `Timeline` writes its lane-label gutter rows' heights inline from the
  same lane heights `timeline.css` draws. One function feeds all three, and a pure function is
  testable, which a media query is not.

**The reference point is 1440 x 900, not the desktop.** `App.openEditor` opens the editor at
`min(1440, availWidth - 120) x min(900, availHeight - 120)`, so on the owner's 4K desktop the editor
viewport *is* 1440 x 900 - which is why that size scores exactly 1 and the big monitor is left
alone. The same arithmetic is why a 1366 x 768 laptop hands this function a 1246 x 648 viewport, not
1366 x 768: the laptop numbers in the test are the sizes to pin, and the real windows land at or
near the floor.

**What deliberately does not scale.** 1px hairlines and dividers, icon `size` props, the 1.5px
strokes on the overlays drawn over the composited video, the 24px hit-target floor
(`--e-ctl-xs`, controls.css's rule 4 - a smaller screen takes type and padding, never a hit target),
and the two controls whose geometry TypeScript mirrors for pointer maths: the Slider
(`Slider.tsx`'s `THUMB = 14`) and the Switch (`Switch.tsx`'s inline thumb `left`).

## MIN_SCALE

```ts
export const MIN_SCALE = 0.86
```

The floor the multiplier clamps to. At 0.86 the 12.5px body rung is 10.75px, which is as small as
this app's chrome may read; below it the discrete steps take over instead.

## NARROW_W

```ts
export const NARROW_W = 1420, TIGHT_W = 1400, CRAMPED_W = 1100
```

The three viewport widths at which a column steps down, exported so a caller (or a future tuning
pass) can name them rather than re-deriving the comparison.

- `NARROW_W` moves the properties sidebar **only** - the stage's cheapest 40px. It sits at 1420,
  UNDER the editor's own 1440-wide default window, on purpose: the window the owner sees on the big
  monitor never steps at all, so the design tuned there stays pixel-identical (owner ruling, 2026-09-15).
- `TIGHT_W` moves the rail, the panel column, the sidebar again and the lane gutter.
- `CRAMPED_W` is the floor step. It exists so the 880 x 560 minimum window is *usable* and not
  merely legal: at it the stage is 303px wide against a 248px panel and a 240px sidebar with both
  columns open, which is the "the stage is still the largest thing on screen" requirement written
  as arithmetic (and asserted in `density.test.ts`).

## SHORT_H

```ts
export const SHORT_H = 820, CRAMPED_H = 640
```

The two viewport heights at which the timeline gives height back to the stage. `SHORT_H` catches
every laptop editor window (648, 744, 780 tall) and misses the 900 the big monitor gets;
`CRAMPED_H` is the 560-minimum floor, and also flips `cramped` on for the width steps, since a
window that short is never a window that wide.

## Density

```ts
export interface Density { scale, rail, railBtn, panel, side, gutter, film, row, audioRow, gap, tracks, narrow, tight, cramped, short }
```

One resolved density. `scale` is the unitless multiplier; the rest are CSS px, and each is a design
decision at each step rather than a proportion of the scale - a column width is not a font size.

| field | full | narrow (<1420) | tight (<1400) | cramped (<1100 or <640 tall) | CSS token |
| --- | --- | --- | --- | --- | --- |
| `rail` | 56 | 56 | 48 | 44 | `--e-rail-w` |
| `railBtn` | 42 | 42 | 36 | 34 | `--e-rail-btn` |
| `panel` | 320 | 320 | 288 | 248 | `--e-panel-w` |
| `side` | 360 | 320 | 288 | 240 | `--e-side-w` |
| `gutter` | 72 | 72 | 60 | 60 | `--e-gutter-w` |

| field | full | short (<820 tall) | cramped | CSS token |
| --- | --- | --- | --- | --- |
| `film` | 80 | 64 | 52 | `--e-film-h` |
| `row` | 32 | 28 | 28 | `--e-row-h` |
| `audioRow` | 22 | 20 | 20 | `--e-audio-h` |
| `gap` | 6 | 5 | 5 | `--e-row-gap` |
| `tracks` | 230 | 168 | 120 | `--e-tracks-h` |

`film` is the filmstrip's **drawn** height only. `filmstripPlan.ts`'s `FILMSTRIP_HEIGHT` stays 80
because that is the height thumbnails are generated and cached at (and Rust's `thumbs::FILMSTRIP_*`
must agree); a short window downsamples a tile it already has rather than asking for a second cache
directory.

## densityFor

```ts
export function densityFor(width: number, height: number): Density
```

The density for a viewport in CSS px. The scale is `min(width / 1440, height / 900)` clamped to
`[0.86, 1]` and rounded to two places - the **shorter** axis wins, since that is the one the layout
is actually short of. The stepped sizes follow the axis they spend: the columns step on width, the
timeline's heights on height. A zero, negative or non-finite size resolves to the floor rather than
throwing, so a pre-measurement read is harmless.

Pinned by `density.test.ts` at the five sizes this pass was written against:

| viewport | scale | flags |
| --- | --- | --- |
| 1366 x 768 | 0.86 | narrow, tight, short |
| 1440 x 900 | 1.00 | none |
| 1536 x 864 | 0.96 | none |
| 1920 x 1080 | 1.00 | none |
| 2194 x 1234 | 1.00 | none |

## densityVars

```ts
export function densityVars(d: Density): CSSProperties
```

The density as the inline custom properties `.editor` carries - `--e-density` unitless, every
stepped size in `px`. The cast inside is the standard one for custom properties, which
`CSSProperties` has no index signature for.

## sameDensity

```ts
export function sameDensity(a: Density, b: Density): boolean
```

Whether two densities would paint identically. `useDensity`'s store notifies only on a real change,
so dragging a window edge across a range that crosses no step re-renders nothing.
