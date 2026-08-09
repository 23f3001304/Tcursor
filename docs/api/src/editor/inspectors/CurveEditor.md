# src/editor/inspectors/CurveEditor.tsx

The shared "Transition Curve" control: the six preset cards from `curves.ts`, where the SELECTED card expands in place across the whole row and becomes a draggable cubic-bezier editor. One component now serves `ZoomInspector`, `LayoutInspector` and `CameraMoveInspector`, each of which previously hand-rolled the same card row.

## CurveEditor

```tsx
export function CurveEditor({ value, onChange, label = "Transition Curve" }: {
  value: string; onChange: (easing: string) => void; label?: string;
}): JSX.Element
```

### Props

- `value: string` - the current easing wire-name: one of the six preset keys, or a custom `cubic(x1,y1,x2,y2)`.
- `onChange: (easing: string) => void` - commits a new wire-name. Callers pass a plain `update_*` op applier; there is no special coalescing here (see Undo below).
- `label` - the field label, so a section like T33's layout Exit block can say "Exit curve".

### Cards

The six presets always render; a seventh **Custom** card renders only while `value` IS a cubic, drawing its live curve. Exactly one card is `on` at a time, and that one is a `<div class="e-curve-card on wide">` (spanning the full grid row, taller SVG, interactive) rather than a `<button>` - handles cannot live inside a button without fighting its click and focus behaviour.

Clicking any unselected preset card commits that preset's key in one click, exactly as before.

### Editing

The expanded card draws the curve from `curveOf(value)` and puts a draggable circle on each control point, joined to its anchor corner by a dashed arm.

- **Drag.** Pointer-down on a handle captures the SVG's bounding rect and the pre-drag control points, then window `pointermove`/`pointerup` listeners take over (so the drag survives leaving the small circle). Each move maps the pointer through `clientToCurve` and updates a local `draft`, so the curve follows the pointer with **no ops in flight**; release commits exactly one `cubic(...)` via `onChange`. Dragging a preset is therefore how a preset becomes custom.
- **Keyboard.** Each handle is `tabIndex=0` `role="slider"` with an `aria-valuetext` of its coordinates; the arrow keys nudge by `0.05` (`nudgeHandle`) and commit immediately.
- **Draft lifecycle.** `draft` is cleared by an effect on `[value]` - i.e. once the committed value comes back through props. If the commit is a no-op (dragged back to where it started) the effect does not fire and the draft simply keeps showing the same curve, so there is no flash either way.

### Undo

No new undo machinery. A drag is one op; repeated drags and arrow nudges land inside `useEditHistory`'s `COALESCE_MS` (400ms) window and fold into a single undo step, the same way a slider drag already does.

### Two subtle React details

- The handle is rendered by a plain **function** (`handle(h)`), not a nested component. A nested component would be a fresh component type every render, so React would remount the circle on each commit and a keyboard nudge would lose focus after its first arrow press.
- The expanded card's key is the constant `"selected"`, not the entry's own key. The first nudge on a preset turns the value custom, which changes WHICH entry is selected; a per-entry key would remount the card at that moment and drop focus mid-edit.

### Styling

`.e-curve-card.wide` (grid-column `1 / -1`, taller SVG, default cursor), `.e-curve-arm` (dashed control arm), `.e-curve-h` (the handle, with `:hover`, `:focus-visible` and `.grab` states). Tokens only; no Motion - the curve must track the pointer exactly, and a spring on a handle reads as input lag.
