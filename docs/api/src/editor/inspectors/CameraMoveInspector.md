# src/editor/inspectors/CameraMoveInspector.tsx

Inspector for the selected camera-move keyframe on the Camera lane (a single-point `t_ms/x/y/size` entry, not a region). Mirrors `ZoomInspector`'s shape - shown in the left panel in place of the tab content while a keyframe is selected.

## CameraMoveInspector

```tsx
export function CameraMoveInspector({ move, dur, onApply, onClose }: {
  move: CameraMove; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### Props

- `move: CameraMove` - the selected keyframe.
- `dur: number` - clip duration (bounds the time input).
- `onApply` - every field change applies an `update_camera_move` op; the delete button applies `remove_camera_move` then closes.
- `onClose` - deselect (clears the selection in `Editor`).

### Behavior

**Fields.** Time (seconds, converted to/from `t_ms`, `NumberField`'s default `unit="s"`), X, Y, and Size (`NumberField` with `unit=""` - no suffix, since these are 0-1 fractions, not a time value) are all `NumberField` (`src/editor/controls/NumberField.tsx`), mirroring `ZoomInspector`. Not a raw `<input type="number">`: that control's `onChange` fires on every keystroke including a momentarily-cleared field (`Number("")` is `0`), which used to collapse the PiP to size 0 mid-edit and could send a negative value the Rust side's u32 deserialization rejected (silently, via `applyOp`'s `catch{}`). `NumberField` has no such intermediate state - `x`/`y`/`size` are clamped to `[0, 1]` (matching the Edit API's own clamp) purely by `NumberField`'s stepper only ever moving by `step`, never by a directly-typed value.

**Transition Curve.** A `<CurveEditor>` (see `CurveEditor.md`): the six shared `CAM_CURVES` as one-click cards, with the selected one expanding into a draggable cubic-bezier editor that commits a custom `cubic(x1,y1,x2,y2)`. The named curves are still pickable straight off the timeline via `CameraLane`'s per-segment popover, which keeps its own compact glyph row.

**Delete.** Removes the keyframe and deselects.
