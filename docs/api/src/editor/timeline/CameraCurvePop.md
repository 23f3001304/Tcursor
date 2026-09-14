# src/editor/timeline/CameraCurvePop.tsx

The Camera lane's transition-curve popover, split out of `CameraLane.tsx` by the editor
width/clipping audit (2026-09-14) when it stopped being an in-flow `div` and became a portal.

## CameraCurvePop

```tsx
export function CameraCurvePop({ anchor, easing, onPick, onDismiss }: {
  anchor: Rect; easing: string; onPick: (key: string) => void; onDismiss: () => void;
}): JSX.Element
```

Six `CURVE_GLYPHS` buttons in a 3-column grid (`.e-campop`), picking the easing **into** the later
of the two keyframes a `.e-camseg` sits between. The glyph for the current `easing` is lit, and the
popover stays open after a pick so curves can be auditioned against the live preview - `CameraLane`
owns both the open/closed state and the `update_camera_move` write.

### Props

- `anchor: Rect` - the clicked segment's **viewport** rect, captured at `pointerdown` in
  `CameraLane` (`e.currentTarget.getBoundingClientRect()`). A rect rather than the element, because
  by the time this places itself the segment may have re-rendered.
- `easing: string` - the destination keyframe's current curve, for the lit glyph.
- `onPick: (key: string) => void` - the chosen `CURVE_GLYPHS` key.
- `onDismiss: () => void` - called when a scroll or a resize moves the lane out from under the
  popover. Pass a stable identity (`CameraLane` wraps it in `useCallback`), or the listeners are
  torn down and re-added on every playhead tick.

### Why it is a portal

*The bug this fixes:* the popover used to be `position: absolute; bottom: calc(100% + 6px)` inside
`.e-camlane`. That wrapper exists to escape `.e-camrow`'s own `overflow: hidden`, but the lane sits
inside `.e-tracks`, which scrolls - and `overflow-y: auto` forces `overflow-x` to clip too. So:

- a keyframe near either end of the clip put half the 170px popover outside the track and it was
  cut off horizontally;
- and when **Camera was the first lane in the stack** - a document with no zoom, FX, layout or speed
  clips, which is every recording before anything has been added to it - a popover opening upward
  out of a 32px row landed entirely above `.e-tracks` and was invisible.

It is now portalled to the editor root (`portalHost`) and placed by `placeStackedCentred`
(`popoverPlace.md`) with `preferAbove`, which keeps the original upward placement, centres it on the
segment, clamps it into the window, and flips it **below** the lane when there is no room above.
`.e-campop` therefore carries no `left`/`bottom`/`transform` of its own - those arrive inline - and
is measured once with `visibility: hidden` before being shown, like `Tooltip` and `Picker`.

### The pointerdown guard it kept

The wrapper still calls `e.stopPropagation()` on its own `pointerdown`. Without it every click on a
curve preset bubbled to `.e-tlbody`, which seeks the playhead, pauses playback and takes pointer
capture - stealing the button's own `onClick` about half the time (bug-sweep-2 Task 8, M3). The
portal does not change that: React events propagate through the React tree, not the DOM one, so the
popover's events still reach `CameraLane`'s ancestors.

### Used by

- `src/editor/timeline/CameraLane.tsx` - mounted while a transition segment is picked.
