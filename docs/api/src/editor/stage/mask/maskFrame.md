# src/editor/stage/mask/maskFrame.ts

The one frame the mask overlay and the mask painter must share: the layout and the camera for an instant, resolved the way `drawCompositeFrame` resolves them.

**Why it exists.** `compositeFrame.ts` paints a mask through `maskDraws(effects, frameLayout, cam, ...)`, where `frameLayout` has the layout segment and the camera move already applied and `cam` is `camAt(track, tOut)`. The drag overlay used to build its box from the raw `p.layout` and a hard-coded identity camera, so the instant a zoom region was live the box and the blur were a factor of the zoom apart - the box still tracked the pointer correctly, in its own wrong space. Rather than have the overlay grow a second copy of that resolution, both sides now call this: the painter inline, the overlay through `useStageMask`. Nothing here is new arithmetic; it is `camAt`, `layoutAt` and `frameCamLayout` in the painter's own order.

## MaskFrameScene

```ts
export interface MaskFrameScene {
  track: CamSample[];
  layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null;
  layoutSegs: LayoutSeg[];
  cameraMoves: CameraMove[];
  zooms: Zoom[];
  zoomSettings: ZoomSettings;
}
```

Everything the resolution reads, and nothing else. It is a structural subset of `StageProps`, so `useStageMask` passes `p` straight in with no adapter object - and a field added to the painter's inputs is a compile error here rather than a silent divergence.

`layout` is the FALLBACK, not the answer: `layoutAt` wins whenever the project has presets, and this is what is used when it has none.

## MaskFrame

```ts
export interface MaskFrame {
  layout: PreviewLayout | null;
  cam: Cam;
}
```

The pair `maskDraws` takes, in its argument order. `layout` is the resolved `frameLayout`, `cam` the sampled camera.

## maskFrameAt

```ts
export function maskFrameAt(s: MaskFrameScene, tOut: number, canvasW: number, canvasH: number,
                            drag: CamPose | null): MaskFrame
```

Resolves the frame for `tOut`, which is OUT-time - the same value `drawCompositeFrame` is handed and the same one `useStageEngine` computes once as `outOf(p.map, p.timeMs)`. Passing in-time here would put the overlay on a different instant than the picture under it on any project with a speed ramp.

### Inputs

- `s: MaskFrameScene` - the document and the settings the resolution reads. *Why a whole scene rather than seven arguments:* the painter reads the same seven through one ref bundle, and a bundle on this side keeps the two call sites comparable at a glance.
- `tOut: number` - the playhead in out-time.
- `canvasW`, `canvasH` - the backing-store size of `.e-canvas`, which is what `layoutAt` fits panels into and what `frameCamLayout` measures the camera panel against. `Stage` derives both from `p.layout?.canvas`, so the overlay and the painter get the same numbers.
- `drag: CamPose | null` - the live camera-drag pose, `frameCamLayout`'s `drag`. *Why a parameter when the only caller passes `null`:* it is what makes this function the painter's resolution rather than an approximation of it, and the test can drive it. The caller's `INVARIANT` block says why `null` is right there: the overlay is gated off in move mode and while arranging, which are exactly the two states that make the painter's `activeCamDraft` non-null.

### Returns

`MaskFrame` - `{ layout, cam }`, ready to spread into `maskDraws` or `fxFrameGeometry`.

### Implementation

1. `cam = camAt(s.track, tOut)`.
2. `base = layoutAt(s.layoutSegs, s.layoutPresets, tOut, [canvasW, canvasH]) ?? s.layout` - the layout-segment blend, falling back to the plain project layout exactly as the painter's `?? r.layoutRef.current` does.
3. `layout = frameCamLayout(base, tOut, cam.scale, s.cameraMoves, drag, s.zooms, s.zoomSettings, canvasW, canvasH)` - the camera keyframe or, absent one, the zoom action's shrink.

*Why `cam.scale` is read before `frameCamLayout` and passed into it:* the camera panel shrinks as a function of how far the zoom has travelled, so the two are not independent. The painter has the same ordering.

### Behaviors

- `resolves the layout and the camera drawCompositeFrame resolves for the same instant` - the test composes `camAt`, `layoutAt` and `frameCamLayout` itself, exactly as `compositeFrame.ts` does, and asserts `maskFrameAt` deep-equals the result on a scene with a presenter layout segment and a 2.2x zoom live at the playhead. It first asserts the fixture is not degenerate: the sampled scale really is 2.2 and the resolved screen panel really does differ from the fallback layout's.
- `lands on the painted rectangle, not on the identity-camera one` - `maskDraws` through this frame equals `maskDraws` through the painter's, and is 2.2x wider than the same call with the camera flattened to identity - the factor the box was out by before. Against the old inputs entirely (the raw `p.layout` AND an identity camera) the span is out by 2.2 x 0.7 and the left edge by more than a hundred canvas pixels. Both ratios are checked to two decimals, not more: `fxFrameGeometry` rounds its crop to whole pixels.

### Used by

- `src/editor/stage/mask/useStageMask.ts` - once per render, memoised on the document, the layout inputs, the playhead and the canvas size.
