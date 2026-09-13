# src/editor/inspectors/InspectorShape.tsx

The one shape every inspector is built from, so the six cannot drift apart row by row. Read top to
bottom, a panel says: **what the clip is** (`PanelHeader`, title + a one-line lede), **when it is**
(a Timing section), **how it looks** (the kind's own sections), **how it moves** (the transition
curve, where the kind has one), then **the destructive action**. `inspectorShape.test.tsx` pins the
section headings in DOM order per inspector, which is the only part of that a later edit can
silently reorder.

The look constraints these primitives encode (from `2026-09-13-m1a-areas.md`'s Look paragraph and
the panel benchmark's sections b/c/e): three planes and no bordered boxes, at most one card level,
one accent per inspector used only for interactive and selected state, one motion language (press
spring 500/30 at scale .96, content swaps on a 0.16s tween), `useReducedMotion` wherever something
glides. The CSS lives in `inspectors.css`, imported by `shell/PropertiesSlot.tsx`.

## secOf

```ts
export const secOf: (ms: number) => number
```

Milliseconds to clip seconds at 2 decimals, the precision every `NumberField` in an inspector reads
and writes. The inspectors all used to define this inline as `sec`.

## clock

```ts
export function clock(ms: number): string
```

`m:ss.d` - the transport's own reading of a moment, used in every lede so a panel's first line is
in the same units as the playhead above it. `0` is `0:00.0`; 12400 is `0:12.4`; 75000 is `1:15.0`.

## spanLede

```ts
export const spanLede: (startMs: number, endMs: number) => string
```

The one-line lede for a clip that occupies a span: `"0:12.4 to 0:15.0, 2.6 s"`. Every span-shaped
inspector (zoom, spotlight, layout, cut, speed) uses it verbatim, so the header always answers
"where am I and how long is this" before any control does. `CameraMoveInspector` is the exception:
a keyframe is a point, so it says `"Keyframe at 0:12.4"`.

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
thing inside - the segmented row's tick, the aim toggle's active tint, the curve handles - reads
that one variable, so the panel and the pill it edits agree and no inspector has two accents.

## Section

```tsx
export function Section({ title, value, children }: { title: string; value?: ReactNode; children: ReactNode }): JSX.Element
```

One section: an 11px `--e-dim` uppercase `<h3>` with an optional right-aligned tabular readout on
the same row, then the controls, at the 20px section rhythm. `value` is the panel's one place for a
**derived** number or state (the preset a zoom currently matches, the time a cut removes, whether a
layout's composition is custom) - it is never an input, which is what keeps the "numeric value on
the label's row" rule from turning into a second control column.

## Hint

```tsx
export const Hint: ({ children }: { children: ReactNode }) => JSX.Element
```

A one-line caption under a section's controls, at hint size in `--e-dim`. Replaces the old
`.e-sec-hint`, which is where the timeline-interaction sentences ("Drag the block on the timeline
to move it") moved when the lede became the span line.

## TimingRow

```tsx
export function TimingRow({ startMs, endMs, durMs, onStart, onEnd }): JSX.Element
```

Start and End as two `NumberField`s on one row, in clip seconds, each clamped by the other (Start's
ceiling is End, End's floor is Start and its ceiling is the clip duration). Every span-shaped
inspector's Timing section is exactly this, so the five cannot disagree about bounds or rounding.

**Why two fields and not three.** The brief asked for start, end and duration on one row "where
they fit". In the 300px properties sidebar a row is 268px wide; a `NumberField` spends 52px on its
two steppers before any digits, so three of them leave ~30px of value each and a reading like
`120.40` collides with the steppers. Duration is therefore reported (live, in the lede, and as the
Timing readout where a kind wants it) rather than typed.

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

`thumbs` switches the row to a 2-up grid whose planes stack a `visual` over the label -
`LayoutInspector`'s preset mini-canvases, each drawing that preset's own resolved panels rather than
an icon of them.

Press feedback is the app-wide spring (500/30, scale .96). `useReducedMotion` drops both the press
and the glide: the tick then renders as a plain span that simply appears on the selected plane.

Every plane gets a `title` (its own label by default), so the row satisfies the benchmark's "no
icon-only control without a tooltip" tell even in the thumbnail variant.

## RemoveButton

```tsx
export function RemoveButton({ label, onClick }: { label: string; onClick: () => void }): JSX.Element
```

The destructive action, always last, always the same shape: a full-width ghost (`.e-del`) in
`--e-dim` that only reddens on hover. It used to be a filled red slab, which made the loudest thing
in a quiet panel the one action a user least often wants. The label is the kind's own wording
("Delete zoom", "Remove cut") and doubles as the tooltip.
