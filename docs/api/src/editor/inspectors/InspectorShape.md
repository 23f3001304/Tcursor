# src/editor/inspectors/InspectorShape.tsx

The one shape every inspector is built from, so the six cannot drift apart row by row. Read top to
bottom, a panel says: **what the clip is and when** (`InspectorHeader`: a lane-coloured dot, the
kind's name, the span on the line under it, and the quiet Delete), then its **sections**, each
behind a `--e-divider` hairline at a 20px rhythm. `inspectorShape.test.tsx` pins the section
headings in DOM order per inspector, which is the only part of that a later edit can silently
reorder.

The look constraints these primitives encode (from `2026-09-13-m1a-areas.md`'s Look paragraph, the
panel benchmark's sections b/c/e, and the owner's 2026-09-14 read of the sidebar): three planes and
no bordered boxes, at most one card level, one accent per inspector used only for interactive and
selected state, one motion language (press spring 500/30 at scale .96, content swaps on a 0.16s
tween), `useReducedMotion` wherever something glides. The CSS lives in `inspectors.css`, imported
by `shell/PropertiesSlot.tsx`.

**What the 2026-09-14 pass changed.** The critique was that TIMING / FRAMING / WEBCAM DURING ZOOM /
FEEL all looked equally important. Three things answer it and all of them live here: sections now
carry a real hairline and 20px of air on both sides of it instead of a bare 20px margin; the header
became the panel's own object rather than the generic `PanelHeader` the seven left-hand panels
share, so a selected clip announces its lane colour and its span before any control; and Delete
left the bottom of the panel for a ghost icon in that header, which is what removed the one
full-width slab from a column that is otherwise all quiet rows.

## secOf

```ts
export const secOf: (ms: number) => number
```

Milliseconds to clip seconds at 2 decimals, the precision every `NumberField` and every `ValueRow`
cell in an inspector reads and writes. The inspectors all used to define this inline as `sec`.

## secText

```ts
export const secText: (ms: number) => string
```

The same 2-decimal clip second, as text with its unit: `5590` is `"5.59s"`, `0` is `"0.00s"`.
Trailing zeros are kept (unlike `secOf`, which is a number and drops them) so a column of these
reads as a column and a value never changes width as it is stepped. `CameraMoveInspector` uses it
directly for its one-point lede, `"Keyframe at 1.20s"`.

Replaces `clock` (`m:ss.d`), which was the transport's reading of a moment. The owner asked for the
header to be in the same units as the fields below it, and every field in an inspector is in
seconds, so the header now is too. The transport still shows `m:ss.d`; that is its own scale.

## spanRange

```ts
export const spanRange: (startMs: number, endMs: number) => string
```

The header's second line for a clip that occupies a span: `"5.59s to 9.58s"`. Every span-shaped
inspector (zoom, spotlight, layout, cut, speed) uses it verbatim, so the header always answers
"where am I" in one reading before any control does. The separator is the word `to`, not a dash of
any width: the codebase has no arrow glyph and an em dash is banned outright.

Replaces `spanLede` (`"0:12.4 to 0:15.0, 2.6 s"`). The length is no longer in the header line at
all: the kinds that want it report it as their Timing section's own right-aligned readout, which is
where every other derived number in a panel already lives.

## InspectorKind

```ts
export type InspectorKind = "zoom" | "fx" | "layout" | "cam" | "cut" | "speed";
```

## InspectorShell

```tsx
export function InspectorShell({ kind, children }: { kind: InspectorKind; children: ReactNode }): JSX.Element
```

The panel root: `.e-panel .e-insp .e-insp-<kind>`. The kind class carries **one** thing, the
`--insp-accent` custom property, set to that clip's own timeline lane color (`--e-zoom`, `--e-fx`,
`--e-layout`, `--e-cam`, `--e-speed`; a cut has no lane color and takes `--e-mut`). Every accented
thing inside - the header dot, the segmented row's tick, the aim toggle's active tint, the curve
handles - reads that one variable, so the panel and the pill it edits agree and no inspector has
two accents.

This is also why `InspectorHeader` takes no `kind` of its own: its dot inherits `--insp-accent`
from this element, so the two can never name different lanes.

## InspectorHeader

```tsx
export function InspectorHeader({ title, range, deleteLabel, onDelete, onClose, thumb }): JSX.Element
```

The inspector's own header, replacing `panels/PanelHeader` in all six (the seven left-hand panels
keep `PanelHeader` unchanged).

- Row one: an 8px `--insp-accent` dot, the kind's name at title rung, an optional `thumb`
  (`LayoutInspector`'s 40x23 arrangement schematic), then two 28px ghost icons.
- Row two: `range`, indented 16px so it sits under the title rather than under the dot, at the
  tabular value rung in `--e-mut`. It carries the full string as its own `title` for the rare span
  that outruns the column.
- `onDelete` is the trash icon, `deleteLabel` its tooltip and `aria-label` ("Delete zoom", "Remove
  cut"). It is `--e-dim` until hovered and only then reddens, using `--e-primary` through
  `color-mix` rather than a colour of its own.
- `onClose` is the X, always labelled "Deselect", and always calls `p.setSel(null)` through the
  inspector - which now also collapses the sidebar (`shell/ClassicShell.md`).

**Why Delete is here and not last.** It used to be `RemoveButton`, a full-width ghost slab at the
bottom of the panel. In a column whose every other row is quiet that slab was the loudest thing on
screen, and it was the one action a user least often wants; the brief's shape puts it in the header
instead, which also leaves the last thing in the panel a section rather than a button.

## Section

```tsx
export function Section({ title, value, children }: { title: string; value?: ReactNode; children: ReactNode }): JSX.Element
```

One section: an 11px `--e-dim` uppercase `<h3>` with an optional right-aligned tabular readout on
the same row, then the controls. The rhythm is 20px above the title, 12px under it, and a
`--e-divider` hairline between two sections with 20px on each side of it (`.e-isec + .e-isec`) -
the first section after the header carries neither the rule nor the extra space.

`value` is the panel's one place for a **derived** number or state (the preset a zoom currently
matches, the length of a zoom's span, the time a cut removes, whether a layout's composition is
custom) - it is never an input, which is what keeps the "numeric value on the label's row" rule
from turning into a second control column.

## Hint

```tsx
export const Hint: ({ children }: { children: ReactNode }) => JSX.Element
```

A one-line caption under a section's controls, at hint size in `--e-dim`. Where the
timeline-interaction sentences ("Drag the block on the timeline to move it") live.

## TimingRow

```tsx
export function TimingRow({ startMs, endMs, durMs, onStart, onEnd }): JSX.Element
```

Start and End as two `NumberField`s on one row, in clip seconds, each clamped by the other (Start's
ceiling is End, End's floor is Start and its ceiling is the clip duration). The Timing section of
every span-shaped inspector **except zoom** is exactly this, so the four cannot disagree about
bounds or rounding.

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
