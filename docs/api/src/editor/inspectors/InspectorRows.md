# src/editor/inspectors/InspectorRows.tsx

The inspector's row widgets - the three controls a section's body is built from (`TimingRow`, `ValueRow`, `SegRow`). Split out of `InspectorShape.tsx`, which keeps the panel frame (`InspectorShell`, `InspectorHeader`, `Section`, `Hint`) and the `secOf`/`secText`/`spanRange` time formatting these rows read; the two files together are the one shape every inspector is built from, and the same look constraints apply here - one motion language (press spring 500/30 at scale .96, exported from `InspectorShape.tsx` as `PRESS`/`PRESS_SPRING` so the header and these rows press identically), `useReducedMotion` wherever something glides, tokens-only colour. `inspectorSections.test.tsx` pins the value-cell stepping and the segment rows; the CSS lives in `inspectors.css`.

## TimingRow

```tsx
export function TimingRow({ startMs, endMs, durMs, minSpanMs = 0, onStart, onEnd }): JSX.Element
```

Start and End as two `NumberField`s on one row, in clip seconds, each clamped by the other (Start's
ceiling is End, End's floor is Start and its ceiling is the clip duration). The Timing section of
every span-shaped inspector **except zoom** is exactly this, so the four cannot disagree about
bounds or rounding.

`minSpanMs` widens that mutual clamp into a floor: Start stops at `endMs - minSpanMs` and End at
`startMs + minSpanMs`, so the row cannot walk a span down to nothing. It is 0 by default, which is
the behaviour every caller but one has, and the one exception is `ClipInspector` (`ClipInspector.md`),
which passes the Clips lane's own `MIN_CLIP_MS` because `update_clip` deletes a clip whose two
bounds meet, the last remaining clip included. A kind whose op tolerates a zero-length span keeps
the default and reads exactly as it did.

**Why two fields and not three.** A `NumberField` spends 52px on its two steppers before any
digits, so three of them in a panel row leave a reading like `120.40` colliding with the steppers.
Duration is therefore reported (live, as the Timing readout where a kind wants it) rather than
typed. `ValueRow` is the shape that does fit three, by dropping the steppers to a 16px pair.

## ValueCell

```ts
export interface ValueCell { label: string; sec: number; min: number; max: number; onChange: (sec: number) => void }
```

One cell of a `ValueRow`: a name, its current value in clip seconds, its own bounds, and where a
change goes. `onChange` receives clip **seconds** (the caller converts to ms), already clamped into
`[min, max]` and rounded to 2 decimals, so a cell can never hand its inspector a negative or an
out-of-span value.

## valueRowColumns

```ts
export const valueRowColumns: (count: number) => number
```

How many columns a `ValueRow` lays its cells out in: one column each up to three, then 2 x 2.

*The bug this fixes* (width audit, 2026-09-14): `.e-ivals` used to be a hard-coded
`repeat(3, 1fr)`, and `ZoomInspector` passes **four** cells (Start, End, In, Out). Four across the
360px sidebar leaves a cell 82px, of which 20 is padding and 16 is the stepper pair - so a
two-minute recording's "125.30s" was clipped by `.e-ivals`' own `overflow: hidden`. Under the old
three-column grid the fourth cell also dropped onto a second row by itself, drawing a left-hand
divider against nothing. Two columns give each cell 164px at 360 and still 128 at the narrowest
sidebar step, which clears the widest reading the row can hold.

## ValueRow

```tsx
export function ValueRow({ cells, ariaLabel }: { cells: ValueCell[]; ariaLabel: string }): JSX.Element
```

Several related numbers as ONE grouped control: a single raised plane, split into equal columns by
`--e-divider` hairlines, each column a label above its value. The column count comes from
`valueRowColumns` and is written inline as `grid-template-columns`, and the hairlines are written
per cell for the same reason - a left edge on every cell that does not start a row, a top edge on
every cell below the first row - so they follow a wrapped row instead of assuming one line. The values are right-aligned and
tabular so the row reads as a column of numbers rather than three separate widgets, and each cell's
steppers are a 16px chevron pair to the right of its value, dim until hovered and disabled at the
cell's own bound. One press is 0.05s, the step every inspector field already uses.

This is what the owner's "controls like 2.8x, 0.35s, 0.45s could have better visual grouping" asked
for: `ZoomInspector`'s Timing section is one of these holding Start, In and Out, which are the three
numbers that describe when a zoom happens and how fast it gets there.

## SegOption

```ts
export interface SegOption { key: string; label: string; on: boolean; disabled?: boolean; title?: string; visual?: ReactNode }
```

## SegRow

```tsx
export function SegRow({ options, onPick, thumbs, ariaLabel }): JSX.Element
```

The one exclusive-choice control in the inspectors: **one raised plane per option**, not buttons
inside a box. The active plane is one lightness step up and carries a 2px accent tick which is a
single Motion element shared by the row (`layoutId`), so picking another option **glides** the tick
there rather than blinking it. The `layoutId` is namespaced by `useId()`, per row instance: two rows
sharing one would fly the tick between them, which a layout segment (Transition curve + Exit Curve)
and `PropertiesSlot`'s swap (both inspectors briefly mounted) would both trigger.

A row where **no** option has `on` renders with nothing selected, which is a real state and not a
bug: `ZoomInspector`'s Feel row sits there whenever the zoom's values match no preset.

`thumbs` switches the row to a 2-up grid whose planes stack a `visual` over the label -
`LayoutInspector`'s preset mini-canvases, each drawing that preset's own resolved panels rather than
an icon of them.

Press feedback is the app-wide spring (500/30, scale .96). `useReducedMotion` drops both the press
and the glide: the tick then renders as a plain span that simply appears on the selected plane.

Every plane gets a `title` (its own label by default), so the row satisfies the benchmark's "no
icon-only control without a tooltip" tell even in the thumbnail variant.
