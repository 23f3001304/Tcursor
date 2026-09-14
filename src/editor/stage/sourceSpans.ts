import type { SourceSpanDto } from "../../lib/ipc";

/** TS mirror of the export's SOURCE SPANS (`export::render::spans`). A mid-take display switch
 *  keeps one encoder canvas and fits the new display into it, so the recorded file carries black
 *  bars from the switch on; the render crops them away by showing only that span's `src` rect, and
 *  the screen panel takes the span's own aspect. This file is the live preview's mirror of that -
 *  which rect to crop the proxy `<video>` by, and how much to squeeze the resolved screen panel.
 *
 *  The PAUSED stage shows the export's own composited frame (`useExactFrame`), so parity there is
 *  the export's by construction; this only has to keep the moving picture honest. */

/** The whole canvas - the `src` of a take that never switched display, and the fallback wherever
 *  spans have not loaded yet. */
export const FULL_SRC: [number, number, number, number] = [0, 0, 1, 1];

/** The span state at output time `t`: the crop rect to show, and the panel-size ratios to apply -
 *  eased across a switch so the panel morphs from the old display's aspect to the new one's over
 *  `transition_ms`, exactly as Rust blends the two spans' resolved scenes. The CONTENT switches to
 *  the new span's rect immediately (Rust pre-blends the old picture INTO that rect); only the
 *  panel shape eases, which is why `src` is never interpolated. */
export interface SpanState { src: [number, number, number, number]; fit: [number, number] }

const lerpN = (a: number, b: number, t: number) => a + (b - a) * t;

/** The index of the span active at `t` - the last one that has started - or -1 when there are
 *  none (spans not loaded). */
export function activeSpanIdx(spans: SourceSpanDto[], t: number): number {
  let idx = -1;
  for (let k = 0; k < spans.length; k++) if (t >= spans[k].start_ms) idx = k;
  return idx;
}

/** Resolve the span state at `t`. `ease` is the caller's easing mirror (`layoutTrack.ts`'s
 *  `ease`), fed "smooth" - the curve Rust's `SpanTrack` uses for a switch - so the two sides bend
 *  the panel on one implementation. Falls back to the full canvas with no squeeze when `spans` is
 *  empty or `t` precedes the first span, which is also exactly what a single-span take resolves to. */
export function spanAt(spans: SourceSpanDto[], t: number, ease: (name: string, p: number) => number): SpanState {
  const i = activeSpanIdx(spans, t);
  if (i < 0) return { src: FULL_SRC, fit: [1, 1] };
  const s = spans[i];
  if (i === 0 || s.transition_ms <= 0) return { src: s.src, fit: s.fit };
  const elapsed = t - s.start_ms;
  if (elapsed >= s.transition_ms) return { src: s.src, fit: s.fit };
  const prev = spans[i - 1];
  const f = ease("smooth", elapsed / s.transition_ms);
  return { src: s.src, fit: [lerpN(prev.fit[0], s.fit[0], f), lerpN(prev.fit[1], s.fit[1], f)] };
}

/** Squeeze a resolved screen-panel rect (`[x, y, w, h]`, fractions of the output) about its own
 *  centre by a span's `fit` ratios, giving the switched-to display its own aspect. The identity
 *  for `[1, 1]`, i.e. for every frame of a take that never switched - which is what keeps such a
 *  take's preview byte-for-byte what it was. */
export function fitPanel(rect: [number, number, number, number], fit: [number, number]): [number, number, number, number] {
  const [x, y, w, h] = rect;
  const [fw, fh] = [w * fit[0], h * fit[1]];
  return [x + (w - fw) / 2, y + (h - fh) / 2, fw, fh];
}

/** A 0..1 CANVAS point (the basis `CamSample`'s cursor and `ClickSample` use) as a 0..1 point
 *  within the screen panel, through the span's crop rect. The TS mirror of Rust's
 *  `coordmap::to_panel`, and the identity for the full canvas. Values outside 0..1 are kept, not
 *  clamped: a point on the other display's picture really is off the panel, and every caller
 *  already clips (`panelClipRect`, the canvas clip path) rather than wrapping. */
export function toPanelFrac(x: number, y: number, src: [number, number, number, number]): [number, number] {
  return [(x - src[0]) / Math.max(src[2], 1e-6), (y - src[1]) / Math.max(src[3], 1e-6)];
}
