# src/editor/panels/CameraMoveField.tsx

The Camera panel's "how it moves" group: the Move-in-preview switch and, directly under it, the only two controls that switch enables. Split out of `CameraPanel.tsx` by the panel pass, which moved this group to the bottom of the panel so that the switch sits above what it enables rather than four groups away from it.

All the keyframe arithmetic lives here; `CameraPanel` is left as a flat read of the panel's flow.

## CameraMoveField

```tsx
export function CameraMoveField({ doc, timeMs, applyOp, moveMode, onMoveModeChange, camDraftRef, staticSize }: {
  doc: EditDoc; timeMs: number; applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean; onMoveModeChange: (v: boolean) => void;
  camDraftRef: RefObject<CamPose | null>; staticSize: number;
}): JSX.Element
```

### Props

- `staticSize` - `ma.cam_size`, the non-keyframed webcam size. Used two ways: as the value the keyframe slider shows wherever no keyframe owns the frame (`camMoveAt` is `null` there), and as the last-resort seed for a keyframe added with nothing drafted and an empty track.
- `camDraftRef` - the pending, un-committed drag pose from the stage. See below.

### Behavior

**Webcam size in Move mode** writes to the `camera_moves` keyframe at the playhead (creating one if none is within the snap window) instead of to the static appearance size. Which is why `CameraPanel` drops `cam_size` from its own Size group while `moveMode` is on: one slider with that name, in the place that currently owns it.

**`setKfSize` deliberately does not clear `camDraftRef`** (review round 2, Important - unlike `addKeyframeHere`). It fires on every slider tick during a drag and reads `camDraftRef.current` each time to preserve a PENDING stage drag's x/y across the whole gesture. Clearing after the first tick would drop that x/y (falling back to `nearestPose`/0.5 on the very next tick, mid-drag) - worse than the narrower known gap it leaves: `camDraftRef.current.size` can go stale against what a slider commit just wrote, so a PiP drag started immediately afterwards seeds `start.size` from the stale value. Pre-existing, not introduced by M6's fix, and not a one-liner to close without the regression above.

**`addKeyframeHere`** saves the drafted pose - or the sampled one if nothing was dragged - as a keyframe at the playhead, and is the ONLY thing that commits: a bare drag never does. It clears `camDraftRef` once used (M6, review round 1 Important 3): the draft exists to survive playback ticks until this action consumes it, and leaving it behind would let a stale pose seed the next drag. The button's label flips to "Update keyframe at playhead" when `camKeyframeAt` finds one already there.

**Seeding outside the span.** `nearestPose` samples the track at the time clamped into `camKfRange`, so adding a keyframe past either end continues from where the track left off instead of jumping to frame-centre. The live layout pose is not available in this panel.

### Used by

- `src/editor/panels/CameraPanel.tsx` - the last group in the panel.
