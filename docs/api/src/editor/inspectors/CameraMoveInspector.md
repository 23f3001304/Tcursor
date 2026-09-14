# src/editor/inspectors/CameraMoveInspector.tsx

Inspector for the selected camera-move keyframe on the Camera lane (a single-point `t_ms/x/y/size` entry, not a region).

Sections, in DOM order (pinned by `inspectorShape.test.tsx`): **Timing**, **Placement**, **Shape**, **Transition** - the shared reading order from `InspectorShape.md`, with Delete in the header rather than last. This is the one inspector whose Timing section is a single field rather than a `TimingRow`, and whose header line is `"Keyframe at 1.20s"` (`secText`) rather than a span, because a keyframe is a point in time and has no length.

## CameraMoveInspector

```tsx
export function CameraMoveInspector({ move, dur, onApply, onClose }: {
  move: CameraMove; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### Props

- `move: CameraMove` - the selected keyframe.
- `dur: number` - clip duration (bounds the time input).
- `onApply` - every field change applies an `update_camera_move` op; the header's delete icon applies `remove_camera_move` then closes.
- `onClose` - deselect (clears the selection in `Editor`).

### Behavior

**Fields.** Time (seconds, converted to/from `t_ms`, `NumberField`'s default `unit="s"`), X, Y, and Size (`NumberField` with `unit=""` - no suffix, since these are 0-1 fractions, not a time value) are all `NumberField` (`src/editor/controls/NumberField.tsx`), mirroring `ZoomInspector`. Not a raw `<input type="number">`: that control's `onChange` fires on every keystroke including a momentarily-cleared field (`Number("")` is `0`), which used to collapse the PiP to size 0 mid-edit and could send a negative value the Rust side's u32 deserialization rejected (silently, via `applyOp`'s `catch{}`). `NumberField` has no such intermediate state - `x`/`y`/`size` are clamped to `[0, 1]` (matching the Edit API's own clamp) purely by `NumberField`'s stepper only ever moving by `step`, never by a directly-typed value.

**Shape.** A `CamShapeField` (`CamShapeField.md`) over `move.shape`/`move.roundness`: the keyframe's own shape, which the webcam morphs between keyframes exactly as it moves between their positions (`cameraMoves.md`). "Layout" keeps whatever the layout underneath uses - the meaning of every keyframe made before shapes existed.

**Transition curve.** A `<CurveEditor>` (see `CurveEditor.md`): the six named curves as a segmented row over one canvas whose two handles shape a custom `cubic(x1,y1,x2,y2)` directly. The named curves are still pickable straight off the timeline via `CameraLane`'s per-segment popover, which keeps its own compact glyph row and draws from the same `curveGlyphs.ts` paths.

**Delete.** Removes the keyframe and deselects.
