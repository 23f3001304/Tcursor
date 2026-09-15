# src/editor/inspectors/CameraMoveInspector.tsx

Inspector for the selected camera-move keyframe on the Camera lane (a single-point `t_ms/x/y/size` entry, not a region).

Sections, in DOM order (pinned by `inspectorShape.test.tsx`): **Timing**, **Placement**, **Shape**, **Motion** - the shared reading order from `InspectorShape.md`, with Delete in the header rather than last. This is the one inspector whose Timing section is a single field rather than a `TimingRow`, and whose header line is `"Keyframe at 1.20s"` (`secText`) rather than a span, because a keyframe is a point in time and has no length.

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

**Fields.** Time (seconds, converted to/from `t_ms`, `NumberField`'s default `unit="s"`), X, Y, and Size (`NumberField` with `unit=""` - no suffix, since these are 0-1 fractions, not a time value) are all `NumberField` (`src/editor/controls/fields/NumberField.tsx`), mirroring `ZoomInspector`. Not a raw `<input type="number">`: that control's `onChange` fires on every keystroke including a momentarily-cleared field (`Number("")` is `0`), which used to collapse the PiP to size 0 mid-edit and could send a negative value the Rust side's u32 deserialization rejected (silently, via `applyOp`'s `catch{}`). `NumberField` has no such intermediate state - `x`/`y`/`size` are clamped to `[0, 1]` (matching the Edit API's own clamp) purely by `NumberField`'s stepper only ever moving by `step`, never by a directly-typed value.

**Shape.** A `CamShapeField` (`CamShapeField.md`) over `move.shape`/`move.roundness`: the keyframe's own shape, which the webcam morphs between keyframes exactly as it moves between their positions (`cameraMoves.md`). "Layout" keeps whatever the layout underneath uses - the meaning of every keyframe made before shapes existed.

**Motion.** `MotionField` (`motion/MotionField.md`, M3, 2026-09-15): the preset row over the graph of the ONE ramp into this keyframe from the previous one (or from the live pose over `KF_BLEND_MS` for the first), with the next keyframe's ramp as a ghost. Its length is the keyframe spacing, so the graph is passed `retimeable: false` and only `easing` is ever written; the Time field above retimes.

**Delete.** Removes the keyframe and deselects.

## camGraphInput

```ts
export function camGraphInput(move: CameraMove, moves: CameraMove[]): GraphInput
```

The keyframe as the graph draws it: lane `"cam"`, `startMs` = the previous keyframe's `t_ms` (else `t_ms - KF_BLEND_MS`, clamped at 0), `endMs` = `t_ms`, one ramp on `easing`, no out ramp, and the next keyframe (from `moves`, the whole lane) as the ghost.
