# src/editor/timeline/Playhead.tsx

## Playhead

```tsx
export function Playhead({ pct, playing, dragging }: {
  pct: number; playing: boolean; dragging: boolean;
}): JSX.Element
```

The timeline playhead: the brand mark turned 90 degrees - a vertical accent stroke with the dot at its head, instead of the triangular scrub handle it used to carry.

### Inputs

- `pct` - the playhead's position as a percentage of the track width.
- `playing` - snaps the position instead of tweening it. A 0.12s tween on a 60fps playback tick would lag the frame it is meant to mark.
- `dragging` - lights the glow and mounts the ripple. `Timeline.tsx` passes real state here, not its high-frequency `scrubbing` ref, so a pointermove burst never re-renders anything.

### The three layers

All three are Motion-owned, because all three are stateful:

- the line's `left%`, tweened while paused and snapped while playing;
- the glow, crossfaded on `dragging` - never lit at rest, so the line stays a plain confident stroke until the user actually grabs it;
- the drag ripple, a ring propagating out from the dot and decaying over ~300ms, mounted only while dragging.

### Reduced motion

The ripple alone is dropped. The playhead still moves, still glows, still reads as grabbed - it just does not pulse. This is what the panel-design benchmark's section (d) asks for specifically: disable only the ripple.

### Why it is its own file

`Timeline.tsx` sits at its line cap, so the playhead's three layers had nowhere to grow. Timeline now renders one element for all of it.

### Styling

`.e-ph` and `.e-ph-glow` keep their timeline layout rules in `editor.css`. The head (`.w-ph-head`, a dot) and the ripple (`.w-ph-ripple`) are wave-motif pieces and live in `src/lib/wave.css` with the rest of the motif. The head is still the one interactive part of an otherwise `pointer-events: none` playhead, and its `pointerdown` still bubbles to `.e-tlbody`'s existing scrub handler with no extra wiring.
