# src/editor/stage/sourceSpans.ts

TS mirror of the export's SOURCE SPANS (`export::render::spans`). A mid-take display switch keeps one encoder canvas and fits the new display into it, so the recorded file carries black bars from the switch on; the render crops them away by showing only that span's `src` rect, and the screen panel takes the span's own aspect. This file is the live preview's mirror of that: which rect to crop the proxy `<video>` by, and how much to squeeze the resolved screen panel.

The PAUSED stage draws the export's own composited frame over the whole canvas (`useExactFrame`), so parity there is the export's by construction. This only has to keep the MOVING picture honest, which is why the panel squeeze rides on ratios Rust computed (`SourceSpanDto.fit`) instead of re-deriving `inset_rect` in TS, and why the live path deliberately does not attempt the cross-dissolve.

## FULL_SRC

```ts
export const FULL_SRC: [number, number, number, number] = [0, 0, 1, 1];
```

The whole canvas - the `src` of a take that never switched display, and the fallback wherever spans have not loaded yet.

## SpanState

```ts
export interface SpanState { src: [number, number, number, number]; fit: [number, number] }
```

The span state at one output instant: the crop rect to show, and the panel-size ratios to apply.

## activeSpanIdx

```ts
export function activeSpanIdx(spans: SourceSpanDto[], t: number): number
```

The index of the span active at `t` - the last one that has started - or -1 when there are none (presets not loaded). Mirrors Rust's `SpanTrack::idx`.

## spanAt

```ts
export function spanAt(spans: SourceSpanDto[], t: number, ease: (name: string, p: number) => number): SpanState
```

Resolve the span state at `t`.

### Inputs

- `spans` - `LayoutPresets.spans`, in order.
- `t` - output ms.
- `ease` - the caller's easing mirror (`layoutTrack.ts`'s `ease`), fed `"smooth"`, the curve Rust's `SpanTrack` uses for a switch. Injected rather than imported so the two modules do not form an import cycle and so there is one easing implementation in TS, not two.

### Behaviors worth knowing

- Empty `spans`, or a `t` before the first span: the full canvas with no squeeze - which is also exactly what a single-span take resolves to at every instant, so a recording that never switched display is unaffected.
- The CONTENT switches to the new span's rect the instant the switch lands (Rust pre-blends the old picture into that rect); only `fit` eases, over `transition_ms`. That is why `src` is never interpolated.
- The squeeze moves monotonically across the transition and settles exactly on the new span's `fit`.

## fitPanel

```ts
export function fitPanel(rect: [number, number, number, number], fit: [number, number]): [number, number, number, number]
```

Squeeze a resolved screen-panel rect (fractions of the output) about its OWN centre by a span's `fit` ratios, giving the switched-to display its own aspect. The identity for `[1, 1]`, i.e. for every frame of a take that never switched - the pin that keeps such a take's preview what it was.

## toPanelFrac

```ts
export function toPanelFrac(x: number, y: number, src: [number, number, number, number]): [number, number]
```

A 0..1 CANVAS point (the basis `ClickSample` uses) as a 0..1 point within the screen panel, through the span's crop rect. The TS mirror of Rust's `coordmap::to_panel`, and the identity for the full canvas.

Values outside 0..1 are kept, not clamped: a point on the other display's picture really is off the panel, and every caller already clips (`panelClipRect`, the canvas clip path) rather than wrapping.

### Used by

- `src/editor/stage/fxGeometry.ts` - `mapCanvas` composes it with the panel/zoom projection.
- `src/editor/timeline/layoutTrack.ts` - `layoutAt` carries the active `src` onto `PreviewLayout`.
