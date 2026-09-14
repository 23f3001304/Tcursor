# src/editor/controls/popoverPlace.ts

Where a floating layer goes once it has been **portalled out of its anchor**, and which element it
is portalled into. Added by the editor width/clipping audit (2026-09-14).

The problem it exists for: clipping in CSS is DOM containment, not geometry. An ancestor with
`overflow: hidden` or `overflow: auto` cuts off an absolutely positioned descendant wherever that
descendant is drawn, and there is no escaping it from inside - not with a big `z-index`, not with
`position: fixed` (a Motion `transform` on any ancestor makes that ancestor the containing block
again and the clip comes straight back). The editor is full of such ancestors: `.e-panel`'s scroll
box, `.e-panel-slot`, `.e-props-side`, `.e-tracks`, `.e-camrow`, and every tile strip. So the layer
moves out and is placed here instead.

Everything in this file is pure, so the flip rules are pinned by `popoverPlace.test.ts` rather than
by opening a menu at the bottom of a panel and looking at it.

## Rect

```ts
export interface Rect { left: number; top: number; width: number; height: number }
```

A viewport-space box: the four fields of `getBoundingClientRect()` that placement actually reads.
Callers hand in the anchor's rect (and `CameraLane` stores one in state, since its anchor has gone
by the time the popover is placed).

## Placement

```ts
export interface Placement { left: number; top: number; flipped: boolean }
```

Where to paint, in viewport coordinates, i.e. straight into a `position: fixed` element's inline
`left`/`top`. `flipped` is true when the preferred side had no room and the opposite one was used;
callers point their enter animation the other way with it, so a menu that opened upward still slides
out of its button rather than dropping onto it.

## EDGE_MARGIN

```ts
export const EDGE_MARGIN = 6
```

How close to the window edge a floating layer may sit. Every result is clamped into
`[EDGE_MARGIN, viewport - EDGE_MARGIN - size]`.

## placeBeside

```ts
export function placeBeside(anchor: Rect, w: number, h: number, vw: number, vh: number, gap = 8): Placement
```

Beside the anchor and vertically centred on it: to the **right** by default, flipped to the left
when the right side would leave the window and the left side has room. What `Tooltip` wants - the
rail sits against the window's left edge, so its labels always take the right side and the flip only
fires for a caller placed near the right edge.

Clamping favours keeping the layer in the window over keeping it centred, so a label taller than the
space above and below its anchor lands at `EDGE_MARGIN` rather than half off the top.

## placeStacked

```ts
export function placeStacked(
  anchor: Rect, w: number, h: number, vw: number, vh: number, gap = 4, preferAbove = false
): Placement
```

Above or below the anchor with their left edges aligned. `preferAbove` picks the starting side and
the other is used when the first does not fit; when neither fits, the **preferred** side wins and
the clamp pulls it into the window, which for a downward menu means the first row stays reachable
rather than the menu hanging off the top.

- `Picker` calls it with the defaults: the menu opens below and flips up near the foot of a panel,
  the inspector column, or a settings dialog.
- `CameraCurvePop` calls it (through `placeStackedCentred`) with `preferAbove`, matching the
  popover's original upward placement, and gets the downward flip for free when the Camera lane is
  the first lane in the stack.

## placeStackedCentred

```ts
export function placeStackedCentred(
  anchor: Rect, w: number, h: number, vw: number, vh: number, gap = 4, preferAbove = false
): Placement
```

`placeStacked` with the layer centred on the anchor instead of left-aligned with it - a popover that
points at a mark (the camera lane's transition segment) belongs over its middle. The horizontal
clamp still wins, so a segment at the very start of the clip gets a popover pinned to the window's
left margin rather than half of one hanging outside it.

## portalHost

```ts
export function portalHost(from: Element | null): Element
```

The element a portalled popover mounts into: **the editor's own root (`.editor`), not
`document.body`**.

*Why not the body:* the whole `--e-*` palette is declared on `.editor` (see `editor.css`'s header),
and custom properties inherit down the DOM. A layer parented to `document.body` would resolve every
colour, radius and shadow it paints to nothing, in both themes.

`.editor` is one node above every overflow container in the editor, so mounting there escapes all of
them; and the layer itself is `position: fixed`, whose containing block is the viewport, so
`.editor`'s own `overflow: hidden` does not clip it either. Falls back to `document.body` for a
caller rendered outside the editor, which is what component tests do.

Callers resolve it once, after mount, into state (`useLayoutEffect`): it needs a mounted node to
walk up from, and a portal container that changed identity between renders would tear the portal
down and rebuild it.

### Used by

- `src/editor/controls/Tooltip.tsx` - `placeBeside`.
- `src/editor/controls/Picker.tsx` - `placeStacked`.
- `src/editor/timeline/CameraCurvePop.tsx` - `placeStackedCentred`.
