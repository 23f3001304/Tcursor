# src/editor/stage/fx/captionDraw.ts

The preview's caption BLIT: the TS mirror of Rust `export::fx::caption::captiondraw`, called once per composite tick from `compositeFrame`. It positions nothing of its own - every number comes from `captionPreview.layoutCaption`, which is the shared half of the parity contract (ADDED-5).

Split out of `captionPreview.ts` in the M5 look pass, along the same line the Rust side already drew: `captionlayout` decides where and how far in, `captiondraw` decides how the pixels land. Keeping the two mirrors one-to-one is what lets a reviewer read the Rust file and the TS file side by side, and it put both back inside the 200-line budget.

This is the one deliberate departure from the "export is the reference, route preview frames through Rust" rule: a backend PNG per frame would make captions lag by a full IPC round trip during playback, which is the one thing a caption must never do. While the playhead rests, `useExactFrame` still paints the export's real frame over the whole composite, so the resting image remains the export's own.

## drawCaptions

```ts
export function drawCaptions(ctx: CanvasRenderingContext2D, w: number, h: number, caps: Caption[],
                             style: CaptionStyle, accent: [number, number, number], t: number): void
```

Draw the caption covering `t` onto the preview canvas - the mirror of Rust `captiondraw::overlay`, argument for argument.

### Inputs

- `ctx` - the stage canvas' 2D context, mid-tick. *Why the raw context:* the tick already owns it, and the draw is bracketed in `save`/`restore` so no state leaks back.
- `w`, `h` - the canvas backing-store size.
- `caps` - `outDoc.captions`, read live through `captionsRef`.
- `style` - `doc.settings.captions`, through `capStyleRef`: the colours, the pill alpha, the animation and its length.
- `accent` - `doc.settings.ui.accent`, through `accentRef`. Used ONLY when `style.highlight_color` is `null`, which is the default and means "follow the interface accent".
- `t` - the instant on the OUTPUT clock (`tOut`), NOT clip time: the caption track is remapped like every other region list.

### Returns

`()`. A no-op when captions are off, between captions, at zero alpha, or on a blank caption - the same four early-outs the export takes.

### Implementation

1. `captionAt`, then `layoutCaption`, then the alpha and empty-lines early-outs.
2. When `style.pill`, a `roundRect` filled `rgba(pill_color, pill_alpha / 100 * alpha)` at radius `min(pillH * 0.3, pillW / 2)`. `pill_alpha: 62` over black reproduces the pre-M5 scrim exactly.
3. `ctx.font = \`600 ${fontPx}px "Inter Variable", system-ui, sans-serif\`` - the app already bundles `@fontsource-variable/inter`, so this is the same face family the export embeds, with no new dependency. `fontPx` already carries the `pop` scale.
4. An alphabetic baseline and a 1 px dark `shadowOffset` matching the export's shadow blit.
5. A `Pen` carries the two resolved colours, the highlight's character range, the `words` reveal cut and `wordAlpha`; each line then goes through `drawLine` with a running character base of `line.length + 1` (the `+ 1` being the space the wrap ate), so a lit or half-revealed word on the second line is addressed correctly.

### Notes

- No Motion here. This is a per-frame canvas draw, not stateful animation - every animation is a pure function of `t`, so there is nothing to spring.
- The draw is NOT unit-tested; a canvas blit has no cheap assertion in jsdom. What is tested is the whole of what it positions against, on both sides, by the `PARITY` fixture. The Rust `captiondraw_tests.rs` covers the pixel consequences (colours, the reveal, the never-moving pill) on the reference renderer.

### Used by

- `src/editor/hooks/stage/compositeFrame.ts` - one call per tick, after the FX overlay blit and before `useExactFrame`'s held export frame.

### Private helpers (all called only by `drawCaptions`)

#### Pen

`interface Pen { alpha, text, lit, hi, cut, wordAlpha }` - everything a line needs that is not its own text or baseline. `text` and `lit` are already CSS `rgb()` strings; `hi` and `cut` are half-open CHARACTER ranges over the caption's joined text, not per-line indices.

#### cutAt

`const cutAt = (cap, shown) => ...` - the `words` reveal boundary: the newest revealed word's character range, or `[0, 0]` when no word has started yet. Mirrors Rust `captiondraw::cut_at`.

#### drawLine

`function drawLine(ctx, line, cx, baseline, pen, base)` - one line of glyphs, centred on the pill's centre x.

The line is measured ONCE at full length and every run is placed from that measurement's left edge, so an unrevealed word still holds its place: the caption is centred on its final width from the first frame and words appearing never reflow the ones already on screen. That is the same invariant the export keeps by advancing `x` past a glyph it did not paint.

With no reveal (`cut === null` - every kind but `words`) the line is drawn in ONE `fillText` and the lit word is repainted over it, which is byte for byte what this file did before the M5 look pass. Only a `words` caption takes the segmented path: the revealed prefix at full alpha, then the newest word at `alpha * wordAlpha`, then the highlight repaint clipped to the revealed part.
