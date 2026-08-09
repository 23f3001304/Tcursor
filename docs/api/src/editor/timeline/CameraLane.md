# src/editor/timeline/CameraLane.tsx

The Camera-move keyframe lane: one diamond marker per `doc.camera_moves` entry (positioned by `t_ms`), a dashed baseline for the track, a translucent bar behind everything showing the span the keyframes actually OWN, and a glowing segment between each consecutive pair where the PiP animates - click a segment to pick that transition's easing curve. Extracted out of `Timeline.tsx` to keep that file under the 200-line limit.

## CameraLane

```tsx
export function CameraLane({ doc, dur, sel, onSel, onApply, track }: {
  doc: EditDoc; dur: number; sel: string | null; onSel: (id: string) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>; track: React.RefObject<HTMLDivElement | null>;
}): JSX.Element
```

Renders an `.e-camlane` wrapper (so the curve popover can escape the row's `overflow:hidden`) around a single `.e-camrow`: the dashed `.e-cambase` baseline, the `.e-camspan` ownership bar behind everything, one `.e-camseg` per consecutive keyframe pair, and every `.e-camkf` diamond on top (no layer-stacking - keyframes are points, not overlapping regions).

### Props

- `doc: EditDoc` - `.camera_moves` is rendered as draggable `.e-camkf` diamonds.
- `dur: number` - clip duration (ms); each diamond's `left` is `(t_ms/dur)*100%`.
- `sel: string | null` / `onSel` - selected keyframe id, lifted to `Editor` (shared with the other tracks' selection).
- `onApply: (op) => Promise<EditDoc | null>` - persists an `update_camera_move` op when a drag ends. `doc.layout` is also read here, as the snap-target source for a keyframe drag.
- `track` - the shared `Timeline` track ref, used to map pointer x into `[0, dur]` the same way `Timeline`'s own `seekAt` does.

### Behavior

**Selection.** Pointerdown on a diamond `stopPropagation`s (so it doesn't also seek the playhead), calls `onSel(id)`, and begins a drag.

**Retime drag.** While dragging, a local draft (`{ id, t_ms }`) tracks the pointer's x-delta converted to ms via the track's bounding width, clamped to `[0, dur]`, then passed through `snapKeyframeMs` (`./camSnap`) against every layout-segment edge (`doc.layout` start/end) and every OTHER keyframe's time; the dragged diamond reads its live position from this draft instead of `doc`. On `pointerup`, the draft commits as one `update_camera_move { id, t_ms }` and clears. Snapping happens in ms space, before the draft is stored, so the diamond, the transition segments, and the span bar all move together already snapped - and the commit writes the snapped value, not the raw one.

**Span bar (Task 27).** `.e-camspan` spans `[first - KF_BLEND_MS, last + KF_BLEND_MS]` of the DRAFT-adjusted keyframe times (so it follows a diamond mid-drag), rendered in the lane's `--e-cam` colour at ~12% alpha. Its `background` is set inline rather than in CSS because the two fade ramps must be exactly `KF_BLEND_MS` wide, which as a percentage depends on the span's own length: `linear-gradient(90deg, transparent 0%, cam .12 R%, cam .12 (100-R)%, transparent 100%)` with `R = KF_BLEND_MS / spanLength`, each stop clamped so a short span (a single keyframe, where the whole bar is the two blend windows) degrades into a symmetric triangle instead of inverting. Zero keyframes -> no bar at all. This is a STATIC fill: plain CSS, no Motion, because nothing about it animates - only its endpoints change, and those change because the underlying data did.

*Why it matters:* the bar is the only place the new ownership rule is visible. Outside it the layout segments own the webcam panel; inside it the keyframes do, with the ramps showing exactly where the handoff happens.

**Mount/hover.** Each diamond fades/scales in on mount and scales up slightly on hover (`motion.div`, matching the zoom/effect/layout pills' `whileHover` treatment), but the drag itself is a manual `pointermove`/`pointerup` window-listener pair - not native HTML5 drag-and-drop and not Motion's own drag gesture.

**Diamond transform.** Motion owns each diamond's `transform` (animating `scale` + `rotate: 45`), so the diamond is centered with a CSS `margin`, never a CSS `transform` - a CSS transform there is clobbered by Motion's inline one (which previously rendered the markers as off-center, un-rotated squares).

**Transition curve.** Clicking a `.e-camseg` opens `.e-campop`, a popover of the shared `CAM_CURVES` glyphs anchored at the segment midpoint; picking one applies `update_camera_move { id, easing }` to the later keyframe (easing eases *into* it) and keeps the popover open so curves can be auditioned against the live preview. It dismisses on Escape or an outside pointerdown (segments excepted, so clicking a neighbour re-anchors instead of closing).
