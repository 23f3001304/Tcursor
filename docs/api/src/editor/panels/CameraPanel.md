# src/editor/panels/CameraPanel.tsx

The Camera rail panel: everything about the picture-in-picture webcam for the default `screen`
layout mode - size, dock margins, shape/roundness, aspect, dock corner, and ring - plus the
Move-mode toggle that lets the user drag the webcam to a free position in the live preview
(writing `camera_moves` keyframes) instead of using the static dock-corner controls.

**Flow (panel pass, 2026-09-13).** Four groups: **Shape** (shape, corner roundness, aspect), **Size and position** (dock location, then the size/margin sliders), **Ring** (the switch and what it enables, in `CameraRingField.tsx`), and **Movement** last (the Move-in-preview switch and, under it, the keyframe size slider and the keyframe button, in `CameraMoveField.tsx`). Every control and every name is unchanged. Previously the Move switch was the FIRST thing in the panel and the controls it enables were the last, with the static appearance controls in between - the panel pass moved it to the bottom so that, like every other switch in the editor's panels, it sits directly above what it turns on.

**Height (usability pass, 2026-09-13).** The four groups came to about 764px against a 620px slot with a rounded shape and the ring on. The **ring** and the **two margin nudges** went under the panel's one `Disclosure`, the margins as a two-up row (`.e-two`: both names are two short words, which is what that class is for). Dock Location is how the webcam is placed; the margins only trim it a few percent off that corner, and the ring is trim on a webcam whose shape, corner and size are already set above - so the disclosure holds trim, and placement stays in sight. Movement moved up one slot to keep the panel's flow intact, and is still the last thing above More.

Measured by rows at 320px, shape Rounded: padding 36, header 53, Shape 184, gap 16, Size and position 140, gap 16, Movement 53, gap 16, the More row 28 - **542** (478 for a circle shape, which has no roundness slider). The one state that still scrolls is Move-in-preview switched ON, which adds its hint, its keyframed size slider and its commit button for 728 - an active editing mode with controls of its own, not the resting panel.

The keyframe arithmetic now lives in `CameraMoveField.tsx` (see `CameraMoveField.md`), which is what keeps this file a flat read of the flow.

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

**Move mode** (now in `CameraMoveField.tsx` - the description below still applies, and `CameraMoveField.md` carries the current detail). The "Move in preview" `.e-switchrow` toggles `moveMode` via `onMoveModeChange`. While on: the static "Webcam size" slider drops out of the Size group (the margins stay, since they still apply wherever no keyframe owns the frame) and is replaced by a keyframed "Webcam size" slider that reads `camMoveAt(doc.camera_moves, timeMs)?.size ?? ma.cam_size` (the keyframed pose at the playhead, falling back to the static size outside any keyframe span) and writes through `commitCamKeyframe` via `setKfSize` - creating a keyframe at the playhead if none is within the snap window - rather than patching `settings` directly. A ghost button ("Add keyframe at playhead" / "Update keyframe at playhead", label depends on `camKeyframeAt`) commits the current drafted-or-sampled pose (`camDraftRef.current ?? kfPose ?? nearestPose`) via `addKeyframeHere`; a bare drag in the preview never commits on its own.

**`addKeyframeHere` clears `camDraftRef` itself (review round 1, Important 3/M6).** After reading `p` from the draft (or falling back to the sampled/nearest pose), it sets `camDraftRef.current = null` BEFORE calling `commitCamKeyframe` - the "explicit action" half of the draft's clear contract (`Stage`/`CamDragHandle` handle the OTHER half, an actual playhead discontinuity - see `playbackTick.md`). Without this, a consumed draft would linger in the ref: since a button press doesn't move `timeMs`, neither `[timeMs]`-keyed effect would ever fire to clear it, and the STALE pose could get silently reused as the seed for the next drag or the next "Add keyframe" press. `CamDragHandle`'s own `[cameraMoves]` effect (content-keyed, not reference-keyed - see `cameraMoves.md`'s `cameraMovesKey`) clears its LOCAL `dragPose` mirror the same moment, since this commit is what changes `doc.camera_moves` in the first place.

**`setKfSize` deliberately does NOT clear `camDraftRef` (review round 2, Important - `addKeyframeHere` and `setKfSize` are NOT symmetric, despite an earlier doc/comment implying they were).** It fires on every pointermove while dragging the "Webcam size" slider, and reads `camDraftRef.current` each time specifically to carry a pending drag's x/y across the WHOLE slider gesture (`d ? { x: d.x, y: d.y, size: v } : { size: v }`) - clearing the ref after the first tick would drop that x/y, falling back to `nearestPose`/`0.5` on the very next tick mid-drag. The tradeoff this leaves: `camDraftRef.current`'s `size` field can go stale relative to what a slider commit just wrote to `doc.camera_moves`, so a PiP drag started right after a slider commit (`CamDragHandle.tsx`'s `onHandlePointerDown`, which seeds `start.size` from `camDraftRef.current ?? sampledPose ?? livePose`) can seed from that stale size instead of the one just committed. Pre-existing (not introduced by the M6 work), narrow, and not closeable as a one-liner without the regression above - left as-is.

**Static sliders.** Iterates `MODE_SLIDERS.screen` filtered to `cam_`-prefixed knobs, minus `cam_radius` (which belongs with the shape that makes it meaningful) and minus `cam_size` while Move mode owns it - so `cam_size`, `cam_margin_x`, `cam_margin_y` normally, `cam_margin_x`/`cam_margin_y` in Move mode. Each is a direct `set(k, v)` on `settings.screen`.

**Shape/roundness/aspect/dock/ring.** Static, non-keyframed appearance, shown in both modes: `Webcam Shape` (circle/rounded/rect), `Corner Roundness` (only when `cam_shape === "rounded"`), `Aspect` (square/16:9), `Dock Location` (BL/BR/TL/TR, only where `MODE_HAS_CORNER` is true - `screen` is one), and the ring controls (delegated to `CameraRingField`, now inside the disclosure). `Margin X` and `Margin Y` are in there too, as a two-up row; a shared `knob(k)` helper renders any of these `SLIDERS` entries, so the size slider and the margin pair are one piece of code in two places.

All four choosers are `Segmented` since the panel pass. Shape already looked like one (a hand-rolled `.e-seg` row); Aspect and Dock Location were dropdowns hiding two and four one-word options. Dock Location's labels stay `BL`/`BR`/`TL`/`TR` - the wording this control has always used, and what fits four-up at 320px - with the full name (`Bottom left`) as each segment's tooltip.

### Notes

- `mode` is hardcoded to `"screen"` - this panel never edits the other four appearance modes (`camera`/`camera_only`/`presenter`/`screen_only`); their editability is routed elsewhere (see the Task 26 audit's settings-coverage matrix).
- `ma = settings.screen || DEFAULT_APPEARANCE.screen` guards against an unset/partial settings blob (e.g. a doc saved before a field existed) the same way every other appearance-reading panel does.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "camera"` panel, wired to `doc.settings.appearance` / `saveDocSettings` plus the move-mode/`camDraftRef`/`applyOp` props threaded from `Editor`.
