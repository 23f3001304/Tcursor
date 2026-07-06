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

**Fields.** Time (seconds, converted to/from `t_ms`), X, Y, and Size are plain number inputs (`x`/`y`/`size` clamped to `[0, 1]` client-side to match the Edit API's own clamp) - not `NumberField`, since that control hardcodes a seconds (`s`) suffix that doesn't fit the 0-1 fraction fields.

**Transition Curve.** The same `e-curve-pick` linear/smooth/spring cards as `ZoomInspector`, applying `{ easing }`.

**Delete.** Removes the keyframe and deselects.
