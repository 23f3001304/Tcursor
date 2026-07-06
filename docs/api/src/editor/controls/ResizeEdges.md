# src/editor/controls/ResizeEdges.tsx

Invisible resize grips for the editor window. The Tauri window is frameless (`decorations: false`) **and** transparent, so the OS draws no resize border and native `startResizeDragging` is silently ignored (its non-client hit-test has nothing to grab). This component instead drives the resize itself with `setSize`/`setPosition` - the window ops that are proven to work here - so the windowed editor can actually be dragged larger/smaller. Rendered once, as the first child of `.editor`, by [Editor](Editor.md).

## ResizeEdges

```tsx
export function ResizeEdges(): JSX.Element
```

Renders the 8 edge/corner grip `<div>`s and runs the pointer-driven resize loop.

### Props

None. The component reads the current window via `getCurrentWindow()` and holds all drag state in refs.

### Behavior

**Grips.**
A module-level `GRIPS` array lists the 8 directions as short codes: `n`, `s`, `e`, `w`, `nw`, `ne`, `sw`, `se`. Each renders a `<div className="e-rz e-rz-{dir}">` carrying `onPointerDown`/`onPointerMove`/`onPointerUp`/`onPointerCancel`. The direction code is decoded by substring: `includes("e")` widens from the right, `includes("w")` from the left, `includes("s")` from the bottom, `includes("n")` from the top - so corners (`nw`, `se`, ...) naturally combine two edges.

**Drag start (`onDown`).**
Ignores non-primary buttons, calls `setPointerCapture` so this grip keeps receiving moves even when the pointer leaves it, then snapshots the window's **logical** geometry into the `drag` ref: position and size come from `outerPosition()` / `outerSize()` (physical) divided by `scaleFactor()`, and the pointer origin is `e.screenX/screenY` (CSS = logical pixels). *Why snapshot once:* every move computes an absolute delta from this origin, which avoids the drift of accumulating per-event movements.

**Drag move (`onMove`).**
Computes `dx/dy` from the captured pointer origin and derives the new `w/h/x/y`. Edges that move the window origin (`w`, `n`) adjust `x`/`y` so the opposite edge stays put; both clamp to `MIN_W = 880` / `MIN_H = 560` (when clamped, the moving edge freezes instead of crossing over). The result is written to a `pending` ref and applied on the next animation frame via `flush`. *Why rAF-batch:* a fast drag fires many pointermove events per frame; coalescing to one `setSize` (+ `setPosition` only when the origin moved) per frame keeps the IPC rate at ~60/s and the resize smooth - the same technique as [morphWindow](../hud/morph.md).

**Drag end (`onUp`).**
Clears the `drag` ref and releases pointer capture (`onPointerCancel` shares the handler so an interrupted drag also resets).

**Positioning (CSS, in `editor.css`).**
`.e-rz` is `position: absolute; z-index: 200`. Edge grips are 6px thick and span their side; corner grips are 13px squares at `z-index: 201` so the diagonal cursor and two-axis resize win where a corner overlaps two edges. Each grip sets the matching resize `cursor`. The grips sit over the window's outer 6px - dead space in the editor chrome (rail margins, panel borders, timeline padding) - so they do not steal clicks from controls; the top 6px clears the 52px top bar's vertically-centred buttons.

### Notes

- Resize uses only `setSize`/`setPosition`, which work on this transparent window where `maximize`/`setFullscreen`/`startResizeDragging` do not. Window resizability itself is still toggled by [App](../App.md) (`setResizable(true)` on open), but the resize motion no longer depends on the OS honoring it.
- The grips live only inside the editor view, so HUD mode is unaffected.
