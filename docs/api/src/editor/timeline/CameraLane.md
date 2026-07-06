# src/editor/timeline/CameraLane.tsx

The Camera-move keyframe lane: one diamond marker per `doc.camera_moves` entry, positioned by `t_ms` rather than a start/end span. Extracted out of `Timeline.tsx` to keep that file under the 200-line limit.

## CameraLane

```tsx
export function CameraLane({ doc, dur, sel, onSel, onApply, track }: {
  doc: EditDoc; dur: number; sel: string | null; onSel: (id: string) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>; track: React.RefObject<HTMLDivElement | null>;
}): JSX.Element
```

Renders a single `.e-camrow` containing every keyframe diamond (no layer-stacking - keyframes are points, not overlapping regions).

### Props

- `doc: EditDoc` - `.camera_moves` is rendered as draggable `.e-camkf` diamonds.
- `dur: number` - clip duration (ms); each diamond's `left` is `(t_ms/dur)*100%`.
- `sel: string | null` / `onSel` - selected keyframe id, lifted to `Editor` (shared with the other tracks' selection).
- `onApply: (op) => Promise<EditDoc | null>` - persists an `update_camera_move` op when a drag ends.
- `track` - the shared `Timeline` track ref, used to map pointer x into `[0, dur]` the same way `Timeline`'s own `seekAt` does.

### Behavior

**Selection.** Pointerdown on a diamond `stopPropagation`s (so it doesn't also seek the playhead), calls `onSel(id)`, and begins a drag.

**Retime drag.** While dragging, a local draft (`{ id, t_ms }`) tracks the pointer's x-delta converted to ms via the track's bounding width, clamped to `[0, dur]`; the dragged diamond reads its live position from this draft instead of `doc`. On `pointerup`, the draft commits as one `update_camera_move { id, t_ms }` and clears.

**Mount/hover.** Each diamond fades/scales in on mount and scales up slightly on hover (`motion.div`, matching the zoom/effect/layout pills' `whileHover` treatment), but the drag itself is a manual `pointermove`/`pointerup` window-listener pair - not native HTML5 drag-and-drop and not Motion's own drag gesture.
