# src/editor/inspectors/LayoutInspector.tsx

Left-panel inspector for the selected timeline layout segment. Every control applies an `update_layout_seg` (or `remove_layout_seg`) op via `onApply`, which persists the doc and bumps the preview, so edits show live.

## LayoutInspector

```tsx
export function LayoutInspector({ seg, dur, onApply, onClose }: {
  seg: LayoutSeg; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### Controls

- **Preset** - `camera` / `presenter` / `screen only` / `camera only`. `"screen"` is deliberately absent: it IS the empty default, reached by deleting the pill or leaving a gap, so offering it as a pill would be a no-op with a UI.
- **Start / End** - the segment's `[start, end)` span, in seconds.
- **Transition** + its curve - the ENTRY cross-fade, which STARTS at `start_ms`.
- **Exit transition** + **Exit Curve** - the exit cross-fade, which COMPLETES at `end_ms`. `0` (the default) is a hard cut. The curve row only appears once the duration is non-zero, since `easing_out` means nothing at `0`.
- **Delete layout**.

### What the Exit section actually does

The exit blends from this segment toward whatever the track resolves after it - the next segment if the two are gapless, else the base `screen` layout - landing on that pose exactly at `end_ms`.

Its visible effect is narrower than it looks, and deliberately so: when a gapless successor has its own entry transition, that entry WINS the overlap and the exit stands down (see `export/scene/layout.md`). Only one blend runs at a time, so back-to-back segments stay seamless. The exit therefore shows up when a segment falls back into a GAP, or hands off to a successor that hard-cuts in - which are exactly the two cases that used to pop.

Both curve rows are the shared `CurveEditor`, so an exit can be hand-drawn as a custom `cubic(...)` just like an entry.
