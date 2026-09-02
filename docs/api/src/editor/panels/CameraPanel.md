# src/editor/panels/CameraPanel.tsx

The Camera rail panel: everything about the picture-in-picture webcam for the default `screen`
layout mode - size, dock margins, shape/roundness, aspect, dock corner, and ring - plus the
Move-mode toggle that lets the user drag the webcam to a free position in the live preview
(writing `camera_moves` keyframes) instead of using the static dock-corner controls.

## CameraPanel

```tsx
export function CameraPanel({ settings, onChange, onClose, doc, timeMs, applyOp, moveMode, onMoveModeChange, camDraftRef }: {
  settings: AppearanceSettings; onChange: (v: AppearanceSettings) => void; onClose: () => void;
  doc: EditDoc; timeMs: number; applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean; onMoveModeChange: (v: boolean) => void; camDraftRef: RefObject<CamPose | null>;
}): JSX.Element
```

### Props

- `settings: AppearanceSettings` - the full 5-mode appearance settings (`doc.settings.appearance`). This panel only ever reads/writes `settings.screen` (see Behavior) - layout mode itself is chosen on the timeline, not here.
- `onChange: (v: AppearanceSettings) => void` - called with the full next `AppearanceSettings` on every control change (via `Editor`'s `saveDocSettings`), same `set()`-helper idiom as every other panel.
- `onClose: () => void` - closes the panel (back to the AI tab).
- `doc: EditDoc` / `applyOp` - needed for `camera_moves`: Move-mode keyframes are edits (`add_camera_move`/`update_camera_move` via `commitCamKeyframe`), not a `settings` patch, so this panel is one of the few that both applies ops AND saves settings.
- `timeMs: number` - the playhead position; Move-mode's "Webcam size" slider reads/writes the keyframe nearest the playhead instead of the static `cam_size`.
- `moveMode: boolean` / `onMoveModeChange: (v: boolean) => void` - whether the preview's drag handle is active. Owned by `Editor` (not local state) so the preview/Stage can react to it too.
- `camDraftRef: RefObject<CamPose | null>` - the live, un-committed drag position from the preview's drag handle (`Stage`). Read here so "Webcam size" and "Add/Update keyframe at playhead" reflect an in-progress drag before it's saved.

### Behavior

**Reset scope (Task 26).** The header's Reset action calls `onChange(resetCameraAppearance(settings))` (`src/hud/preferences/appearanceFields.ts`) - **not** `onChange(DEFAULT_APPEARANCE)`. `resetCameraAppearance` writes only the 8 fields this panel actually shows on `settings.screen` - `cam_size`, `cam_margin_x`, `cam_margin_y`, `cam_shape`, `cam_radius`, `cam_aspect`, `cam_corner`, `cam_ring` - from `DEFAULT_APPEARANCE.screen`, leaving `screen.pad`/`screen.screen_size`/`screen.screen_radius` (owned by `BackgroundPanel`) and the other four appearance modes (`screen_only`/`camera`/`camera_only`/`presenter`) completely untouched. A prior bug called `onChange(DEFAULT_APPEARANCE)` directly here, silently resetting all five modes on every press. The reset action lives in `PanelHeader`'s `onReset` (no separate bottom button, matching Background/Cursor).

**Move mode.** The "Move in preview" `.e-switchrow` toggles `moveMode` via `onMoveModeChange`. While on: the static size/margin sliders are replaced by a single "Webcam size" slider that reads `camMoveAt(doc.camera_moves, timeMs)?.size ?? ma.cam_size` (the keyframed pose at the playhead, falling back to the static size outside any keyframe span) and writes through `commitCamKeyframe` via `setKfSize` - creating a keyframe at the playhead if none is within the snap window - rather than patching `settings` directly. A ghost button ("Add keyframe at playhead" / "Update keyframe at playhead", label depends on `camKeyframeAt`) commits the current drafted-or-sampled pose (`camDraftRef.current ?? kfPose ?? nearestPose`) via `addKeyframeHere`; a bare drag in the preview never commits on its own.

**`addKeyframeHere` clears `camDraftRef` itself (review round 1, Important 3/M6).** After reading `p` from the draft (or falling back to the sampled/nearest pose), it sets `camDraftRef.current = null` BEFORE calling `commitCamKeyframe` - the "explicit action" half of the draft's clear contract (`Stage`/`CamDragHandle` handle the OTHER half, an actual playhead discontinuity - see `playbackTick.md`). Without this, a consumed draft would linger in the ref: since a button press doesn't move `timeMs`, neither `[timeMs]`-keyed effect would ever fire to clear it, and the STALE pose could get silently reused as the seed for the next drag or the next "Add keyframe" press. `CamDragHandle`'s own `[cameraMoves]` effect (content-keyed, not reference-keyed - see `cameraMoves.md`'s `cameraMovesKey`) clears its LOCAL `dragPose` mirror the same moment, since this commit is what changes `doc.camera_moves` in the first place.

**`setKfSize` deliberately does NOT clear `camDraftRef` (review round 2, Important - `addKeyframeHere` and `setKfSize` are NOT symmetric, despite an earlier doc/comment implying they were).** It fires on every pointermove while dragging the "Webcam size" slider, and reads `camDraftRef.current` each time specifically to carry a pending drag's x/y across the WHOLE slider gesture (`d ? { x: d.x, y: d.y, size: v } : { size: v }`) - clearing the ref after the first tick would drop that x/y, falling back to `nearestPose`/`0.5` on the very next tick mid-drag. The tradeoff this leaves: `camDraftRef.current`'s `size` field can go stale relative to what a slider commit just wrote to `doc.camera_moves`, so a PiP drag started right after a slider commit (`CamDragHandle.tsx`'s `onHandlePointerDown`, which seeds `start.size` from `camDraftRef.current ?? sampledPose ?? livePose`) can seed from that stale size instead of the one just committed. Pre-existing (not introduced by the M6 work), narrow, and not closeable as a one-liner without the regression above - left as-is.

**Static sliders (non-Move-mode).** Iterates `MODE_SLIDERS.screen` filtered to `cam_`-prefixed knobs (`cam_size`, `cam_margin_x`, `cam_margin_y` for this mode), each a direct `set(k, v)` on `settings.screen`.

**Shape/roundness/aspect/dock/ring.** Shown in BOTH modes (unlike the size/margin sliders above) since they're static, non-keyframed appearance: `Webcam Shape` (circle/rounded/rect segmented control), `Corner Roundness` (only when `cam_shape === "rounded"`), `Aspect` (square/16:9 picker), `Dock Location` (BL/BR/TL/TR picker, only for modes where `MODE_HAS_CORNER` is true - `screen` is one), and the ring controls (delegated to `CameraRingField`).

### Notes

- `mode` is hardcoded to `"screen"` - this panel never edits the other four appearance modes (`camera`/`camera_only`/`presenter`/`screen_only`); their editability is routed elsewhere (see the Task 26 audit's settings-coverage matrix).
- `ma = settings.screen || DEFAULT_APPEARANCE.screen` guards against an unset/partial settings blob (e.g. a doc saved before a field existed) the same way every other appearance-reading panel does.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "camera"` panel, wired to `doc.settings.appearance` / `saveDocSettings` plus the move-mode/`camDraftRef`/`applyOp` props threaded from `Editor`.
