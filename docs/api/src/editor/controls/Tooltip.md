# src/editor/controls/Tooltip.tsx

A hover label for an icon-only control. Built for the rail (whose seven buttons were icons and nothing else - the owner's "the left toolbar icons need tooltips and hover states"), but it takes any child, so anything else icon-only can name itself the same way.

## TOOLTIP_DELAY_MS

```ts
export const TOOLTIP_DELAY_MS = 350
```

The hover dwell before the label appears. Long enough that sweeping down the rail never lights a trail of labels behind the pointer, short enough that stopping on a button answers "what is this?" well before the native `title` would.

## Tooltip

```tsx
export function Tooltip({ label, children, className }: { label: string; children: ReactNode; className?: string }): JSX.Element
```

Wraps `children` in a `.e-tipwrap` span and, after `TOOLTIP_DELAY_MS` of hover, fades a `.e-tip` label in beside it.

### Props

- `label: string` - the text. Also what the caller should keep in its own `title` attribute: this is the fast, styled path and the native tooltip is the accessible fallback, so both are wanted (`Rail` passes the same string to both).
- `children: ReactNode` - the control, rendered untouched. The wrapper adds no box of its own (`display: inline-flex`; the rail additionally pins it to `flex: none`).
- `className?: string` - appended to the wrapper, for a caller that needs to place it.

### Behavior

**Timing.** `mouseenter` starts a `setTimeout`; `mouseleave` and `pointerdown` clear it and hide (a click should not leave a label hanging over whatever the click just changed). A pending timer is also cleared on unmount - a rail button can unmount mid-hover.

**Focus shows it at once,** with no delay: a focused control has already been deliberately reached, and a keyboard user has no way to "dwell".

**Motion, not CSS.** The label is a `motion.span` inside `AnimatePresence` (opacity plus a 4px `x`, a 0.12s tween), per the app-wide rule that stateful UI animation is Motion's job and CSS keyframes are not used for it. The `x` points the other way once the label has flipped to the left of its control, so it always slides out of the control rather than into it.

**Portalled, not inline** (width/clipping audit, 2026-09-14). The label is rendered through `createPortal` into the editor root (`portalHost`, `popoverPlace.md`) and positioned in viewport coordinates by `placeBeside` - to the right of the control, vertically centred on it, flipping to the left near the window's right edge and clamped `EDGE_MARGIN` off every edge.

*The bug this fixes:* drawn inside `.e-tipwrap`, the label was cut off by any ancestor with `overflow: hidden` or `overflow: auto`. That was first seen inside a horizontally scrolling tile strip, but the rail, `.e-panel`'s scroll box, `.e-panel-slot` and the inspector column are all overflow containers too, so every caller was one placement away from the same thing. Clipping is DOM containment, not geometry, so moving the label out is the only fix that holds; `position: fixed` alone would not, because a Motion `transform` on an ancestor (the rail button's own press spring, the panel slot's swap) makes that ancestor the containing block again.

**Measured before it is shown.** Placement needs the label's own box, so the portalled span mounts once with `visibility: hidden`, is measured in a `useLayoutEffect` (before paint) and re-rendered at its real coordinates. There is no visible frame at the window's corner, and the guard on the already-computed placement is what stops that effect looping.

**`.e-tip`** (`stage.css`) therefore carries no `left`/`top`/`transform` of its own - those arrive inline - only the pill's paint, `position: fixed`, `pointer-events: none` (so it can never swallow the click it is describing) and a `z-index` above the panels and the timeline but below the modals, which the rail cannot be reached through anyway.

### Used by

- `src/editor/shell/Rail.tsx` - all seven panel buttons.
