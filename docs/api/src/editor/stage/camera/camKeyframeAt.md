# src/editor/stage/camera/camKeyframeAt.ts

Shared "is there already a keyframe at the playhead?" helper for the `camera_moves` editing paths that write to *the current keyframe* rather than always adding a new one: the Move-mode panel's size slider and shape picker, and its Add/Update keyframe button (`CameraMoveField.tsx`). Keeping this in one place means all of them agree on the snap window and the update-vs-add decision.

## CAM_KF_SNAP_MS

```ts
export const CAM_KF_SNAP_MS = 60;
```

Frames within this many milliseconds of the playhead are treated as "the same keyframe" - about a frame or two at 30fps, so a slightly-off-exact click still lands on the intended keyframe instead of stacking a near-duplicate a few ms away.

## CamKfPatch

```ts
export interface CamKfPatch { x?: number; y?: number; size?: number; shape?: CamMoveShape; roundness?: number }
```

The fields one commit may set. `shape`/`roundness` are the keyframe's own shape (`"layout"` = inherit the layout's webcam shape); a brand-new keyframe leaves them to the backend's defaults when the patch does not carry them, so a size-only commit never invents a shape.

## camKeyframeAt

```ts
export function camKeyframeAt(moves: CameraMove[], timeMs: number): CameraMove | null
```

### Inputs

- `moves: CameraMove[]` - `doc.camera_moves`, in any order.
- `timeMs: number` - the current playhead time.

### Returns

The `CameraMove` whose `t_ms` is closest to `Math.round(timeMs)` and within `CAM_KF_SNAP_MS`, or `null` if none qualifies.

### Implementation

Linear scan tracking the closest candidate within the snap window (`Math.abs(m.t_ms - t) <= CAM_KF_SNAP_MS`, keeping the smallest distance) - `camera_moves` lists are small (user-authored keyframes), so no sorting/binary-search is warranted.

## commitCamKeyframe

```ts
export async function commitCamKeyframe(
  moves: CameraMove[],
  timeMs: number,
  patch: CamKfPatch,
  fallback: { x: number; y: number; size: number },
  onApply: (op: EditOp) => Promise<EditDoc | null>,
): Promise<EditDoc | null>
```

Commits a `camera_moves` edit at the playhead: updates the existing keyframe within the snap window (patching only the given fields) via `update_camera_move`, else creates one via `add_camera_move` using `patch` for the fields the caller is setting and `fallback` for the rest.

### Inputs

- `moves` / `timeMs` - passed straight to `camKeyframeAt` to find the target keyframe.
- `patch: CamKfPatch` - the field(s) this call is actually changing (e.g. the button commits `{ x, y, size }`; the size slider `{ size }`; the shape picker `{ shape }`).
- `fallback: { x, y, size }` - the full pose to use when creating a brand-new keyframe, so a caller that only cares about one field doesn't have to know the others when there's nothing to update yet (e.g. the size slider passes the *current* x/y as the fallback so a first keyframe doesn't jump the PiP to `(0,0)`).
- `onApply: (op: EditOp) => Promise<EditDoc | null>` - the shared edit-op applier (`Editor.tsx`'s `applyOp`).

### Returns

Whatever `onApply` returns - the updated `EditDoc`, or `null` on failure.

### Implementation

1. `camKeyframeAt(moves, timeMs)` - if found, `onApply({ op: "update_camera_move", id: existing.id, ...patch })`.
2. Else `onApply({ op: "add_camera_move", t_ms: Math.round(timeMs), x: patch.x ?? fallback.x, y: patch.y ?? fallback.y, size: patch.size ?? fallback.size, shape?, roundness? })` - the two shape fields only when the patch carries them.

### Notes

- `moves` is whatever the caller rendered with, which can be one IPC round-trip stale during a slider drag, so two quick commits could both decide "add". The backend folds an add within the same `CAM_KF_SNAP_MS` window of an existing keyframe into an in-place update (`EditOp::AddCameraMove`, Rust `region::CAM_KF_SNAP_MS` = 60), touching only the fields the op carried, so that race cannot stack duplicates or reset a shape - which mattered, because `camMoveAt` prefers the LAST coincident keyframe and the stale duplicate would have won.

### Used by

- `src/editor/panels/camera/CameraMoveField.tsx` - every Move-mode commit: the size slider and shape picker (each a partial patch, with a pending drag's x/y folded in), and the Add/Update keyframe button (the full pose).
