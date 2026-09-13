# src/editor/inspectors/CurveEditor.tsx

The transition curve, as ONE compact block: `EasingPicker`'s segmented row of named curves, then a
single 96px canvas drawing the selected curve, then the two spring sliders when the curve is a
spring. Used everywhere an inspector has a transition: `ZoomInspector` (Feel), `LayoutInspector`
(Transition and Exit Curve) and `CameraMoveInspector` (Transition).

### Why it went and why it is back

The first version was a grid of six curve cards where the selected card expanded in place into a
drag-the-dots editor. The owner vetoed that: a grid of cards inside a panel is the "bordered boxes
stacked at every level" tell the panel benchmark calls out, and an editor that appears by growing
one card makes the panel jump. It was replaced by a plain dropdown - and the owner then asked where
the curve editor had gone. What was wrong was the **grid and the expanding card**, not the ability
to shape a curve.

So the pieces came back (`curveMath.ts` and its tests, restored from history) behind a different
control: no cards, no grid, no popover. One row of names, one canvas that is always the same size in
the same place, and the handles simply live on it whenever the selected curve is a cubic. The canvas
is a plane one step lighter than the panel with no border, matching every other surface in the
inspector.

## CurveEditor

```tsx
export function CurveEditor({ value, onChange, label = "Transition curve" }: {
  value: string; onChange: (easing: string) => void; label?: string;
}): JSX.Element
```

### Props

- `value: string` - the easing wire-name: one of the six preset keys, a parameterised
  `spring(stiffness,damping[,mass])`, or a `cubic(x1,y1,x2,y2)`.
- `onChange` - commits a new wire-name. Callers pass a plain `update_*` op applier.
- `label` - the field label on the row, so a layout segment's exit block can say "Exit Curve". It
  also namespaces the row's shared tick element and the handles' `aria-label`s.

### The canvas

A named curve draws its own published glyph path from `timeline/curveGlyphs.ts` - the same path the
timeline's transition popover draws for that key. A custom cubic, or any curve mid-drag, draws
`curvePath` from the live control points. A spring draws `springPathOf`, sampled from the real
oscillator.

The SVG uses `preserveAspectRatio="none"` so the 100x160 curve box stretches to the panel's width;
every stroke is `vector-effect: non-scaling-stroke` so that stretch never changes a line's weight,
and the two handles are HTML dots positioned over the canvas (`handlePct`) rather than SVG circles,
which the stretch would turn into ellipses.

### Editing

Each handle is an 8px accent dot joined to its anchor corner by a 1px arm. Dragging one writes a
`cubic(...)`, which turns the row's selection to **Custom** - exactly the old behaviour and the old
math. A drag holds the in-flight curve in local state so the line follows the pointer without one op
per `pointermove`, and commits **once** on release; repeated drags inside `useEditHistory`'s coalesce
window still fold into a single undo step. The draft clears when the committed value comes back.

Handles are `role="slider"` with an `aria-valuetext` of both coordinates, and arrow keys nudge by
0.05 (`nudgeHandle`), so the curve is editable without a pointer.

The handle renderer is a plain function, NOT a nested component: a nested one would be a new
component type every render, so React would remount the dot on each commit and a keyboard nudge
would lose focus after its first arrow press.

### Springs

A spring is not a cubic, so handles would silently convert it. When `springOf(value)` matches, the
canvas shows the sampled oscillator with no handles and `SpringControls`' two sliders appear under
it instead (see `SpringControls.md`).

### Motion

The handles animate to their position on a spring (420/34), which is what makes a preset click read
as the dots *moving* to the new curve rather than teleporting. `useReducedMotion` drops that to an
instant position change. Nothing else in the block animates.
