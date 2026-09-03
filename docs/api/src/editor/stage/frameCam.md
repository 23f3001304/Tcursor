# src/editor/stage/frameCam.ts

The single place the preview decides what drives the webcam PiP on a given frame, mirroring the ORDERING inside `FrameRenderer::step_camera` (`src-tauri/src/export/render/mod.rs`).

**Why it is its own module:** the ordering is the parity-critical part - a keyframe override and the smart zoom action must never both apply. Keeping the decision in one function means the preview cannot drift from the export, and it keeps `useCompositeLoop` (which was over the 200-line limit with this inline) focused on the render loop. `activeCamDraft` sits here for the same reason: it is a rule about which draft outranks which, which is this module's whole subject.

## activeCamDraft

```ts
export const activeCamDraft: (draft: CamPose | null, arranging: boolean) => CamPose | null
```

The Move-mode draft the composite should HONOUR this frame - the draft itself normally, `null` while stage arrange mode owns the stage. `useCompositeLoop` wraps `dragPoseRef.current` in it before handing it to `frameCamLayout` below.

**Why it exists.** `frameCamLayout` gives the drag pose precedence over the base layout rect (`drag ?? camMoveAt(...)`), and the base layout rect is exactly where arrange mode's own live draft lives (`arrange/useArrangeDrag.md`). Without this gate, an unsaved Move-mode drag carried into arrange mode pinned the composited webcam while the arrange frame moved freely.

**Why it SUPPRESSES rather than discards.** `CameraPanel` promises the user exactly two things that throw an unsaved Move drag away: pressing Add/Update, or moving the playhead. Selecting a layout segment is not one of them, and it is reachable in ordinary use (the timeline stays interactive with the Camera panel open) with no feedback at the moment of loss, because `EditorPanels` swaps to `LayoutInspector` in the same pass. So nothing here writes `camDraftRef`: the draft is invisible to the composite for as long as arranging lasts and reasserts intact on exit. An entry seek (`arrangeSeekMs`) can still clear it - but that is the documented playhead trigger doing its own job, not arrange mode taking a third one. Pinned by `frameCam.test.ts`'s "entering AND leaving leaves a pending draft intact" and its end-to-end pass through `frameCamLayout`.

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

- **A keyframe or drag is active** - `overrideCamPanel` replaces the rect and scales radius + ring width by the height ratio, so a circle stays round. `live` (the panel's own centre + height, i.e. the layout-resolved pose this frame) is what `camMoveAt` eases out of entering the keyframe span and back into leaving it. Since Task 27 a keyframe is active only INSIDE that span; outside it `camMoveAt` is `null` and this branch is skipped, so the smart zoom action below applies as if there were no keyframes. A live Move-mode drag still wins unconditionally, span or not.
- **Otherwise** - the smart webcam-on-zoom action, and ONLY while the cam panel is narrower than the screen panel (`cam[2] < screen[2]`), the same guard the export applies so big-camera layouts are never shrunk. `resolveCamAction` returns `[action, targetScale]` - the WINNING ZOOM'S OWN scale, not the global `zoom.target_scale` - and that pair feeds `applyCamZoomAction` together, mirroring `export/render/mod.rs`'s `step_camera` (which threads `cam_action_at`'s returned `(action, target_scale)` the same way). Passing the global scale here would have reproduced the export-side bug in preview: a 1.6x zoom would only ever reach ~50% shrink/hide against a 2.2x global default. The same `[action, targetScale]` pair also feeds `camZoomAlpha` (`camZoomAction.ts`), multiplied onto `base.camAlpha` - `applyCamZoomAction` only ever touches geometry, so without this the "Hide" action would fade the PiP in the export but leave it fully opaque in the preview.

### Behaviors worth knowing

- **Keyframes win.** The action is skipped entirely whenever an override is active. In the export this fixed a real bug where `shrink_camera` ran on top of an override, silently scaling a hand-keyframed webcam during zooms.
- Because `camMoveAt` returns a pose for ANY non-empty track (it holds before the first and after the last keyframe), adding a single camera keyframe suppresses the smart action for the whole clip. That is the intended "keyframes win" semantic, not an edge case.
