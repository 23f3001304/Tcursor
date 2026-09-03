# src/editor/stage/arrange/useArrangeDrag.ts

Arrange mode's gesture layer: the live pose draft the composite loop draws, and the single op each gesture commits on release. Called unconditionally by `Stage` (with `seg: null` when the mode is off), so the hook order never changes.

## useArrangeDrag

```ts
export function useArrangeDrag({ seg, presets, canvasRef, canvasW, canvasH, dirtyRef, onApply }: {
  seg: LayoutSeg | null; presets: LayoutPresets | null;
  canvasRef: RefObject<HTMLCanvasElement | null>; canvasW: number; canvasH: number;
  dirtyRef: RefObject<boolean>; onApply: (op: EditOp) => Promise<EditDoc | null>;
}): {
  panels: Panels | null; presets: LayoutPresets | null;
  guideX: number | null; guideY: number | null; active: PanelKind | null;
  onPanelDown: (e: React.PointerEvent, panel: PanelKind, corner: Corner | null) => void;
  onHideCam: () => void;
}
```

### The draft has no drawing path of its own

The dragged pose is folded straight back into `LayoutPresets.segs` (`withDraftSeg`) - the per-segment resolved-rect channel L2 already built - and `Stage` hands the composite loop THAT object instead of the raw presets. So `layoutAt` renders a dragged segment through exactly the code path a committed arrangement takes, cross-fades at its edges included, and neither `layoutAt` nor `useCompositeLoop` needed a line of draft-specific logic. Same "presentation-only, committed by an explicit action" contract as `camDraftRef` (`CamDragHandle.md`), reached without a second override channel.

### Where the gesture starts

From the segment's own STEADY-STATE resolved panels (`resolvedPanelsFor(seg, presets)`), not from `layoutAt` at the playhead. Inside a transition those two differ - `layoutAt` is mid-cross-fade - and committing a blended rect would bake the transition into the segment's pose. The frames therefore show what the segment IS, which is also what the drag edits.

The start pose is `poseOfRect` of that rect, so a segment with no arrangement yet drags from its provenance preset's placement with nothing to seed first.

### Drag lifecycle

`attachPointerGesture` (`../pointerGesture.ts`) owns the window listeners, so `pointerup` and `pointercancel` both end the gesture exactly once and neither can leak a listener; the returned `detach` is stashed in a ref and called from an unmount effect for the case the mode is left mid-drag. `pastDragThreshold` gates the first move, so a plain click on a frame selects nothing new and commits nothing.

The draft holds the WHOLE resolved pair, not just the panel under the pointer, and every gesture starts from what is on screen (`panels`) rather than from the freshly-resolved `base`. Both are the same fix for the same window: a second drag begun before the previous commit's refetch has landed would otherwise snap its panel - or the other one - back to a pre-drag rect for a few frames.

Each `pointermove` maps the pointer to a frame fraction (`mapPointerToCamFraction` - the PiP is an un-cropped top-layer overlay, so this is the plain fraction of the displayed frame, not un-projected through the zoom), runs `movedPose` or `resizedPose` with a `SnapTol` measured once at pointerdown from the canvas' DISPLAYED rect (`SNAP_PX / width|height`, so "8 stage-px" is 8 pixels on screen at any scale) or `null` when Alt is held, stores the resulting rect as the draft, and sets `dirtyRef` so a PAUSED composite loop repaints. On release the guides drop and, if the gesture ever passed the threshold, ONE `setArrangementOp` goes through `onApply`. One op per gesture: the history's 400ms coalesce window is what groups a burst of them into a single undo step (`useEditHistory.md`).

### When the draft is dropped

When the segment's RESOLVED `segs` entry changes reference - i.e. once the `preview_layouts` refetch that the commit triggered has landed - or when the arranged segment changes.

Deliberately NOT when the doc comes back: `applyOp` sets the new doc first and bumps `rev`, and the refetch resolves some frames later. Clearing on the doc would snap the panel back to its pre-drag rect for exactly that window. Keying on the resolved entry hands over seamlessly, and because the TS clamps mirror the Rust ones, every commit really does change that entry.

### onHideCam

The cam frame's own hide affordance - `setArrangementOp(seg, base, "cam", null)`, which on a bare segment also carries the screen's current pose so the write is never "hide both". Only the CAM gets this on the stage; the screen is hidden from `LayoutInspector` instead, so one stray click can never leave a blank frame with nothing left to grab.
