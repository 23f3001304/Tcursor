# src/editor/stage/frameCam.ts

The single place the preview decides what drives the webcam PiP on a given frame, mirroring the ORDERING inside `FrameRenderer::step_camera` (`src-tauri/src/export/render/mod.rs`).

**Why it is its own module:** the ordering is the parity-critical part - a keyframe override and the smart zoom action must never both apply. Keeping the decision in one function means the preview cannot drift from the export, and it keeps `useCompositeLoop` (which was over the 200-line limit with this inline) focused on the render loop.

## frameCamLayout

```ts
export function frameCamLayout(
  base: PreviewLayout | null, t: number, scale: number,
  moves: CameraMove[], drag: CamPose | null, zooms: Zoom[], zoom: ZoomSettings,
  ow: number, oh: number,
): PreviewLayout | null
```

Returns `base` with its `cam` panel replaced by whatever governs this frame.

### Inputs

- `base: PreviewLayout | null` - the layout resolved for this frame (already cross-faded across any layout-segment boundary). Returned untouched when it has no camera panel.
- `t: number` - playhead time in milliseconds.
- `scale: number` - the current zoom scale from `camAt`. *Why passed in:* the caller already computed it for the base draw; recomputing risks the two disagreeing.
- `moves: CameraMove[]` - `doc.camera_moves`.
- `drag: CamPose | null` - the live Move-mode pointer pose, or null. *Why it outranks the sampled track:* the PiP must follow the cursor without writing to the backend every frame; the drag only commits a keyframe on release.
- `zooms: Zoom[]` / `zoom: ZoomSettings` - resolve the smart action (per-zoom override, then global default).
- `ow` / `oh` - the preview canvas's pixel dims (its backing store, sized from `PreviewLayout.canvas`), the basis `PreviewLayout` fractions use.

### Returns

- **A keyframe or drag is active** - `overrideCamPanel` replaces the rect and scales radius + ring width by the height ratio, so a circle stays round. `staticPose` (the panel's own centre + height) acts as the implicit `t=0` keyframe a lone keyframe eases in from.
- **Otherwise** - the smart webcam-on-zoom action, and ONLY while the cam panel is narrower than the screen panel (`cam[2] < screen[2]`), the same guard the export applies so big-camera layouts are never shrunk.

### Behaviors worth knowing

- **Keyframes win.** The action is skipped entirely whenever an override is active. In the export this fixed a real bug where `shrink_camera` ran on top of an override, silently scaling a hand-keyframed webcam during zooms.
- Because `camMoveAt` returns a pose for ANY non-empty track (it holds before the first and after the last keyframe), adding a single camera keyframe suppresses the smart action for the whole clip. That is the intended "keyframes win" semantic, not an edge case.
