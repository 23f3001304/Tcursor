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

**The row widgets moved.** `TimingRow`, `ValueRow` and `SegRow` - the three controls a section's
body is built from - live in `InspectorRows.tsx` (`InspectorRows.md`). This file is the frame: the
panel, the header, the section and the hint, plus the time formatting all of them read. The press
vocabulary is exported from here rather than duplicated there, so the header's icon buttons and the
rows' segments press identically.

## PRESS

```ts
export const PRESS: { scale: number }
```

The one press target every inspector control shares, `scale: 0.96`. Passed as `whileTap`, and
dropped to `undefined` under `useReducedMotion` or on a disabled control.

## PRESS_SPRING

```ts
export const PRESS_SPRING: { type: "spring"; stiffness: number; damping: number }
```

The spring that press runs on, 500/30 - critically damped enough to have no visible bounce, which is
what makes a press read as a press and not as a wobble. `InspectorRows.tsx` imports both rather than
redeclaring them.

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
export type InspectorKind = "zoom" | "fx" | "layout" | "cam" | "cut" | "speed" | "caption";
```

## InspectorShell

```tsx
export function InspectorShell({ kind, children }: { kind: InspectorKind; children: ReactNode }): JSX.Element
```

The panel root: `.e-panel .e-insp .e-insp-<kind>`. The kind class carries **one** thing, the
`--insp-accent` custom property, set to that clip's own timeline lane color (`--e-zoom`, `--e-fx`,
`--e-layout`, `--e-cam`, `--e-speed`; a cut has no lane color and takes `--e-mut`, and a caption takes `--e-wave`, the editor's one audio hue). Every accented
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
