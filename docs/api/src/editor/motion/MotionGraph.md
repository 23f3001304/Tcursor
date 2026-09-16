# src/editor/motion/MotionGraph.tsx

The region's whole motion as one picture, and the surface you edit it on. Curves are drawn, not named: the Transition section of the zoom, layout and camera-move inspectors becomes a preset row (`PresetRow`) above this graph, and the words "Smooth", "In / Out" and "Feel" retire from the UI (`docs/superpowers/specs/2026-09-15-motion-editor-design.md` 4).

Everything drawn here comes from `buildGraph`, reached through `useGraphDrag` (which adds the live draft and the selection). This file holds no easing math and no second opinion about where a key sits - it is layout, and that is why the graph cannot disagree with the render.

**What it draws, in paint order:** the two value rails and their labels, the ms ticks, the ramps as shaded bands with the hold flat between them, the follow hint under that hold when the zoom's target is the cursor, the neighbours' handoffs faint in the gutters, the curves, and last the keys with their tangent arms.

**Retired by this file:** `CurveEditor.tsx`, `EasingPicker.tsx` and `curveMath.ts`, whose two-control-point cubic could only ever describe one segment of one ramp.

### The stylesheet: src/editor/motion/motion.css

The graph's own stylesheet, imported here because this is the only thing that draws it (no docs/api mirror of its own: the validator maps a doc to a `.ts`/`.tsx`/`.rs` source, and stylesheets are documented where they are imported).

One accent per graph: `--m-accent` is set from the lane class (`.e-mg-zoom` / `.e-mg-layout` / `.e-mg-cam`) to `--e-zoom` / `--e-layout` / `--e-cam`, the same tokens the region's timeline pill and its inspector already borrow, so a curve visibly belongs to the thing it edits. Everything else is the panel's quiet vocabulary - `--e-divider` rails, `--e-dim` ghosts and ticks - because the only thing on this canvas that should catch the eye is the motion.

Sizes INSIDE the svg are viewBox geometry, not type-scale sizes: the graph is drawn 320x140 and laid out at whatever width the panel gives it, so a tick label is 9 user units rather than a `--e-text-*` token, exactly as the dots are `r=4`. Reduced motion is honoured in the sheet (`prefers-reduced-motion` drops the dot and handle transitions); the graph itself has no animation to suppress, since it redraws on state.

## LANE_LABEL

```ts
const LANE_LABEL = { zoom: "Zoom", layout: "Layout", cam: "Camera move", text: "Text" }
```

What a screen reader hears the graph called. The lane's name plus what can be done with it ("drag a key to shape it"), because an svg of paths announces nothing useful on its own.

## MotionGraph

```tsx
export function MotionGraph(props: {
  input: GraphInput; onCommit: (patch: GraphPatch) => void;
  readOnly?: boolean; retimeable?: boolean; className?: string;
}): JSX.Element
```

`input` is the region in the model's terms and `onCommit` takes the patch back (see `graphModel.md` and `useGraphDrag.md` for both). `readOnly` draws the motion with nothing to grab - Settings' default-motion preview, and any ramp whose curve is a spring. `retimeable` is false for a camera move, whose ramp length is the keyframe spacing.

Two details that are easy to get wrong and are therefore deliberate here:

- **The key nodes are a render FUNCTION, not a nested component.** A nested component is a new component type on every render, so React would remount every dot on each commit and a keyboard nudge would lose focus after its first arrow press. `CurveEditor` paid for that lesson once.
- **A tangent handle is only drawn where one exists.** The in handle belongs to the segment ENDING at its key and the out handle to the one starting there, so the first key's in and the last key's out - the two the wire form writes as `0 0` and the evaluator never reads - are never shown, and neither are the handles of a hold or a linear segment, which have no tangents to shape.
