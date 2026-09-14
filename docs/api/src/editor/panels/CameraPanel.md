# src/editor/panels/CameraPanel.tsx

The Camera rail panel: **what the webcam does over time**. The Move-mode toggle that lets the user drag the webcam to a free position in the live preview (writing `camera_moves` keyframes), the keyframed size slider, and the commit button - all of it delegated to `CameraMoveField.tsx`.

**What moved out (2026-09-14).** Size, shape, corner roundness, aspect, dock corner, the two margin nudges and the ring used to live here - but only for the `screen` layout, which meant the other four layouts' webcams were not editable from the editor at all. They are all in the **Layouts** panel now (`LayoutsPanel.md`), one layout at a time, so there is exactly one place that answers "how does the webcam look in this layout". This panel keeps the one thing that is not per-layout: a camera move is a keyframed pose on the output frame, and it applies over whatever layout is on screen underneath it.

The panel is now header, the move field, and one hint line - "Size, shape, position and ring live in Layouts." One line rather than a button: the rail is two icons away, and a panel that navigates to another panel would be a second navigation system. The hint is wrapped in an `.e-grp` purely so it inherits the panel's 16px section rhythm.

## CameraPanel

```tsx
export function CameraPanel({ settings, onClose, doc, timeMs, applyOp, moveMode, onMoveModeChange, camDraftRef }: {
  settings: AppearanceSettings; onClose: () => void;
  doc: EditDoc; timeMs: number; applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean; onMoveModeChange: (v: boolean) => void; camDraftRef: RefObject<CamPose | null>;
}): JSX.Element
```

### Props

- `settings: AppearanceSettings` - **read, never written** now. One number is taken from it: `settings.screen.cam_size`, the static size a pose falls back to outside any keyframe span, which is what `CameraMoveField`'s `staticSize` has always been seeded from. `?? DEFAULT_APPEARANCE.screen` guards a doc saved before a field existed, the same way every appearance-reading panel does.
- `onClose: () => void` - collapses the panel column (`EditorPanels` passes `setTab(null)`; see `EditorPanels.md`).
- `doc: EditDoc` / `applyOp` - `camera_moves` keyframes are edits (`add_camera_move` / `update_camera_move` via `commitCamKeyframe`), not a settings patch.
- `timeMs: number` - the playhead. The keyframed "Webcam size" slider reads and writes the keyframe nearest it.
- `moveMode` / `onMoveModeChange` - whether the preview's drag handle is active. Owned by `Editor` (not local state) so the Stage can react to it too.
- `camDraftRef: RefObject<CamPose | null>` - the live, un-committed drag position from the preview's drag handle.

**Gone:** `onChange: (v: AppearanceSettings) => void`. This panel no longer writes appearance at all, so `EditorPanels` stopped passing it. `PanelHeader`'s `onReset` went with it - there is nothing left here to reset, and the per-layout reset is "Reset this layout" in the Layouts panel (`resetLayout`, `layoutPresets.md`). The old `resetCameraAppearance` helper in `hud/preferences/appearanceFields.ts` was deleted along with its spec, since `resetLayout` is the same idea generalized to all five layouts.

### Behavior

Everything below is `CameraMoveField.tsx`'s - see `CameraMoveField.md` for the current detail; the notes here are the ones that were written against this file and still apply.

**Move mode.** The "Move in preview" `.e-switchrow` toggles `moveMode`. While on, a keyframed "Webcam size" slider reads `camMoveAt(doc.camera_moves, timeMs)?.size ?? staticSize` (the keyframed pose at the playhead, falling back to the static size outside any keyframe span) and writes through `commitCamKeyframe` via `setKfSize` - creating a keyframe at the playhead if none is within the snap window - rather than patching `settings`. A ghost button ("Add keyframe at playhead" / "Update keyframe at playhead") commits the current drafted-or-sampled pose (`camDraftRef.current ?? kfPose ?? nearestPose`); a bare drag in the preview never commits on its own.

**`addKeyframeHere` clears `camDraftRef` itself (review round 1, Important 3/M6).** After reading `p` from the draft (or falling back to the sampled/nearest pose) it sets `camDraftRef.current = null` BEFORE calling `commitCamKeyframe` - the "explicit action" half of the draft's clear contract (`Stage`/`CamDragHandle` handle the other half, an actual playhead discontinuity - see `playbackTick.md`). Without this, a consumed draft would linger: a button press does not move `timeMs`, so neither `[timeMs]`-keyed effect would fire to clear it, and the stale pose could be reused as the seed for the next drag.

**`setKfSize` deliberately does NOT clear `camDraftRef`.** It fires on every pointermove while dragging the size slider and reads `camDraftRef.current` each time specifically to carry a pending drag's x/y across the whole gesture; clearing after the first tick would drop that x/y mid-drag. The tradeoff left standing: `camDraftRef.current.size` can go stale relative to a slider commit, so a PiP drag started right after one can seed from the stale size. Pre-existing, narrow, and not closeable as a one-liner without that regression.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "camera"` panel, wired to `doc.settings.appearance` (read-only) plus the move-mode / `camDraftRef` / `applyOp` props threaded from `Editor`.
