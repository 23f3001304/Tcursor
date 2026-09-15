# src/editor/stage/transport/viewMode.ts

How the composited frame sits in the stage area: Fit (the long-standing behaviour), Fill and 100%. The choice is session-only UI state - nothing is written to `edit.json`, nothing is read back at startup, and reopening the editor starts at Fit again.

**Why a module store and not `useState`.** The control (`ViewPicker`, in the transport's right group) and the thing it sizes (`Stage`) are siblings whose only shared parent is `shell/ClassicShell.tsx`. A prop channel through the shell is the textbook answer and is what this should become the day anything else needs to read the mode; until then the store keeps the pair working without widening `Transport`'s prop list or the shell's. It is deliberately tiny: one `let`, one `Set` of subscribers, and a `useSyncExternalStore` reader.

## ViewMode

```ts
export type ViewMode = "fit" | "fill" | "native"
```

`"fit"` is the default: the whole frame, uncropped. `"fill"` cover-crops it to the stage area. `"native"` is the 100% view - one canvas pixel per CSS pixel, centred, scaled DOWN to fit a small window but never up.

## VIEW_MODES

```ts
export const VIEW_MODES: { id: ViewMode; label: string; title: string }[]
```

The three options in the order `ViewPicker` renders them, with the short chip label ("Fit", "Fill", "100%") and the sentence that becomes each button's `title`.

## setViewMode

```ts
export function setViewMode(next: ViewMode): void
```

Sets the mode and notifies every mounted reader. A no-op when the mode is unchanged, so a repeat click on the active option does not re-render the stage.

## useViewMode

```ts
export function useViewMode(): ViewMode
```

Subscribes the calling component to the store (`useSyncExternalStore`). Used by `Stage` (to size its box) and by `ViewPicker` (to mark the active option).

## stageFrameStyle

```ts
export function stageFrameStyle(view: ViewMode, canvasW: number, canvasH: number, frameW: number, frameH: number): CSSProperties
```

The stage's inline box for one view mode, given the stage area's measured content box in CSS px. Pure, and pinned by `viewMode.test.ts` rather than by dragging a window.

- **Fit**, and any mode before the area has been measured (`frameW`/`frameH` at 0), returns just `{ aspectRatio }` - the CSS sizer in `stage.css` (`max-width`/`max-height` at 100%) already produces the largest uncropped box, and it needs no measurement to do it.
- **Fill** returns the COVER box: `max(frameW, frameH * ar)` wide. That is larger than the area on one axis on purpose - `.e-stagewrap`'s `overflow: hidden` crops it, and `.e-stagewrap.fill > .e-stage` sets `flex: none` so flex-shrink cannot quietly undo the crop. `.e-stagewrap.fill` also drops the 12px padding, so Fill really does reach the area's edges.
- **100%** returns `min(canvasW, frameW, frameH * ar)` wide: the canvas' own pixel size, clamped down to the fit box so a 4K canvas in a small window is scaled to fit rather than overflowing, and never scaled up past 1:1.

`maxWidth`/`maxHeight` are set to `"none"` in the two measured modes, since the stylesheet's `100%` caps would otherwise clamp the explicit width straight back to the area.

**The invariant this protects.** The stage box stays exactly the canvas' displayed box in all three modes. `CamDragHandle`, `ZoomReticle`, `ArrangeOverlay` and the cut/zoom click mapping all position against that box (as a % of `.e-stage`, or via `getBoundingClientRect`), so a mode that cropped the canvas INSIDE the stage - `object-fit: cover`, say - would silently offset every one of them. Fill crops by moving the whole box past the area instead, which `getBoundingClientRect` still reports in full.

## useFrameSize

```ts
export function useFrameSize(ref: RefObject<HTMLElement | null>): [number, number]
```

The element's live content-box size via one `ResizeObserver`, as `[width, height]`, re-rendering only when a dimension actually changes. Returns `[0, 0]` until the first observation and wherever `ResizeObserver` does not exist at all (jsdom) - which `stageFrameStyle` reads as "stay on the Fit path", so a test that mounts `Stage` gets the plain aspect-ratio box instead of a crash.
