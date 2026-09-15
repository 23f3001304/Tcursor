# src/editor/stage/StageOutline.tsx

The one thing the AI review sheet draws outside the AI panel: a thin accent box around the region a proposal named, while that proposal is being previewed.

## StageOutline

```tsx
export function StageOutline({ rect, canvasW, canvasH, layout, track, tOut }: {
  rect: [number, number, number, number] | null;
  canvasW: number; canvasH: number; layout: PreviewLayout | null;
  track: CamSample[]; tOut: number;
}): JSX.Element
```

### Behavior

**A marker, never a mode.** `pointer-events: none` and `aria-hidden`: the canvas underneath keeps its click-to-zoom exactly as it was, and nothing about the stage changes state while an outline is up.

**It tracks the picture.** `outlineRect` (`director/review/outline.md`) re-projects the region through the camera crop sampled at `tOut` on every render, so the box follows a zoom as it ramps instead of floating over it, and disappears on its own once the crop pushes the region off frame.

**Always mounted, conditionally filled.** `Stage` renders it unconditionally and passes `rect: null` for "nothing" - that is what keeps the `AnimatePresence` exit alive, so the outline fades out instead of blinking away. Reduced motion drops both the fade and the enter.

**The dim is a shadow.** `box-shadow: 0 0 0 9999px` punches the region out of a 22% black wash, clipped by `.e-stage`'s own `overflow: hidden` - one element rather than four, and a literal black in both themes for the same reason the modal scrim is one (dimming is a shadow, not a colour). The edge itself is `--e-ai`, the director's own hue.

### Used by

- `src/editor/stage/Stage.tsx` - from the `outline` prop, which `ClassicShell` fills from `ShellProps.stageOutline`.
