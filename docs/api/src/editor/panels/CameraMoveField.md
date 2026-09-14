# src/editor/panels/CameraMoveField.tsx

The Camera panel's "how it moves" group: the Move-in-preview switch and, directly under it, the controls that switch enables - the keyframed webcam size and shape at the playhead, and the button that commits a drag as a keyframe. Split out of `CameraPanel.tsx` by the panel pass, which moved this group to the bottom of the panel so that the switch sits above what it enables rather than four groups away from it.

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

**Webcam size and Shape in Move mode** write to the `camera_moves` keyframe at the playhead (creating one if none is within the snap window) instead of to the static appearance. Size is the slider; Shape is a `CamShapeField` (`CamShapeField.md`) reading the keyframe found by `camKeyframeAt` (`"layout"` when none is there yet), so a shape pick on an empty instant creates the keyframe carrying it. Both go through one `commit(patch)`.

**`commit` keeps a pending drag, and keeps it CURRENT.** It fires on every slider tick and reads `camDraftRef.current` each time, folding the draft's x/y into the keyframe so a drag in progress is not lost mid-gesture. It also writes the committed size back into the draft (`{ ...d, size }`). *Why that matters (the bug fixed 2026-09-14):* the composite draws `camDraftRef` in PREFERENCE to the keyframe (`frameCamLayout`'s `drag ?? camMoveAt(...)`), so a draft left holding the pre-slider size kept the PiP at that size no matter what the slider wrote - after any drag, the size slider looked dead. Updating the draft in step closes that gap, and also means a PiP drag started right after seeds `start.size` from the size just committed rather than a stale one. The draft is still never CLEARED here - only by the button below or by moving the playhead.

**`addKeyframeHere`** saves the drafted pose - or the sampled one if nothing was dragged - as a keyframe at the playhead, and is the ONLY thing that commits a drag: a bare drag never does. It clears `camDraftRef` once used (M6, review round 1 Important 3): the draft exists to survive playback ticks until this action consumes it, and leaving it behind would let a stale pose seed the next drag. The button's label flips to "Update keyframe at playhead" when `camKeyframeAt` finds one already there.

**Seeding outside the span.** `nearestPose` samples the track at the time clamped into `camKfRange`, so adding a keyframe past either end continues from where the track left off instead of jumping to frame-centre. The live layout pose is not available in this panel.

### Used by

- `src/editor/panels/CameraPanel.tsx` - since 2026-09-14, essentially the WHOLE panel: everything else there was per-layout appearance and moved to Layouts (`CameraPanel.md`), leaving the header, this field, and one hint line. `staticSize` still comes from `settings.screen.cam_size`.
