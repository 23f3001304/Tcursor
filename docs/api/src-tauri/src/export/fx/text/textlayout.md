# src-tauri/src/export/fx/text/textlayout.rs

Where a text item sits, how big it is, and how far into its animation it is: one pure function over an item, an accent, an output size and an instant.

It is pure and it is shared, in the sense that matters: `src/editor/stage/text/textPreview.ts` implements the identical contract, and the two are held together by a generated 48-row table that both sides carry BYTE FOR BYTE. That is the whole parity mechanism. Nothing in this file touches a frame, a font file, or a renderer.

**The block is measured from CHARACTER COUNTS, not from font metrics**, at `captionlayout`'s fixed advance `ADV`. That is deliberate and it is what makes the mirror possible: the preview runs in a browser and cannot load the export's `ab_glyph` face, so if the box depended on real Inter advances the two sides could never agree. Instead the shared function decides the BOX from counts, and each side then centres its OWN truly measured run inside that box (`textdraw::run_x` with `glyph::run_width`, `textDraw.ts::runX` with `ctx.measureText`). That split is ADDED-5, first made for captions and reused here unchanged.

### The two re-exported constants

```rust
pub use crate::export::fx::caption::captionlayout::{ADV, LINE_H};
```

`ADV` (0.52) is the nominal character advance as a fraction of the font size, and `LINE_H` (1.32) the line height as a multiple of it. They have no `##` sections of their own, following the convention the rest of `docs/api` uses for re-exports (`export/types.md` documents none of the `Key`/`Keys` or `Overlay*` names it re-exports either): the section belongs on the page of the file that DECLARES the symbol, which for these two is `caption/captionlayout.md`.

They live in `captionlayout` because captions landed first, and spec 4's fit point 3 rules that whichever feature landed first owns a shared constant. Text re-exports rather than redefines them so there is one number, and so a change moves captions and text together instead of silently desynchronising them. 0.52 is an average over Inter's lowercase; it is not correct for any individual glyph and does not have to be, because it only ever sizes a box that each side then centres its real measured run inside.

## SUB_RATIO

```rust
pub const SUB_RATIO: f32 = 0.58;
```

The second line's size as a fraction of the main line's. A lower third's role sits distinctly under its name without becoming a footnote.

## PAD_X

```rust
pub const PAD_X: f32 = 0.60;
```

The plate's horizontal padding on EACH side, as a fraction of the main font size. The block therefore widens by `2 * font_px * PAD_X` when the style plates, and the run starts half that in from the block's left edge.

## PAD_Y

```rust
pub const PAD_Y: f32 = 0.34;
```

The plate's vertical padding above and below, as a fraction of the main font size. Smaller than `PAD_X` because `LINE_H` already carries leading inside the text rows, so equal padding would read as bottom-heavy.

## MARGIN

```rust
pub const MARGIN: f32 = 0.060;
```

The safe-area inset from the frame edge for an EDGE anchor, as a fraction of output HEIGHT. Height for both axes on purpose: a margin that is 6 percent of width on a 1080x1920 portrait export and 6 percent of height on a 1920x1080 landscape one would look like two different designs. Pinning both to height makes the inset the same optical size at any aspect. A centre anchor ignores it entirely.

## RULE_W

```rust
pub const RULE_W: f32 = 0.006;
```

The `bar` style's rule width as a fraction of output height. The block reserves `2 * oh * RULE_W`: one width for the rule and one for the gap between the rule and the run.

## SLIDE_FRAC

```rust
pub const SLIDE_FRAC: f32 = 0.040;
```

How far a `slide` animation travels at `g = 0`, as a fraction of output height. Four percent is a nudge, not an entrance across the frame: the item is already where it belongs and the slide is only there to give the eye a direction of arrival.

## POP_FROM

```rust
pub const POP_FROM: f32 = 0.86;
```

The scale a `pop` animation starts from. It grows to 1 as `g` goes to 1, so the item never overshoots past its final size; a pop that went over 1 and settled back would need a spring and would fight the item's own easing.

## LaidText

```rust
pub struct LaidText { main, sub, font_px, sub_px, plate, main_baseline, sub_baseline, rule,
                      alpha, reveal, scale, shift, fill, shadow, plate_rgb, plate_alpha,
                      rule_rgb, centred }
```

One laid-out text item at one instant: everything `textdraw` needs to paint it and nothing it needs to think about.

- `main: String`, `sub: Option<String>` - the TRIMMED strings. `sub` is `None` both when the item has no second line and when its second line trims to nothing, so the blit never has to re-check.
- `font_px`, `sub_px` - pixel sizes, `scale` ALREADY APPLIED to both.
- `plate: [f32; 4]` - the scrim rect `[x, y, w, h]`, or `[0; 4]` when the style does not plate.
- `main_baseline`, `sub_baseline` - `[x, baseline_y]`. `x` is the LEFT of the text column; a centred item's blit shifts its own measured run inside the box from there. The `y` is a BASELINE, matching `glyph::draw_run`'s convention and the canvas's `textBaseline = "alphabetic"`.
- `rule: [f32; 4]` - the `bar` rule's rect, or `[0; 4]`.
- `alpha: f32` - the whole item's opacity, 0..1. Multiplies the plate's own alpha, the rule, and both runs.
- `reveal: usize` - how many characters of `main` to draw, or `usize::MAX` for all of them. The TypeScript twin uses `number | null` with `null` meaning all, because JavaScript has no `usize::MAX`; the pinned table encodes it as `-1` on both sides.
- `scale`, `shift` - the pop scale and the slide offset, reported for inspection and for the pinned table; both are already folded into the sizes and the positions, so a painter never applies them again.
- `fill`, `shadow`, `plate_rgb`, `plate_alpha`, `rule_rgb` - the style's decisions, already resolved against the accent, so the painter never needs the accent itself.

  `fill` is the GLYPH colour, `rule_rgb` the bar's. They are different on purpose and the difference is the `bar` style's whole point: spec 5.4's table gives `bar` a white fill and a `ui.accent` rule, so a lower third reads as white type with the project's colour beside it. `rule_rgb` is always the accent (the rule is only ever drawn when `rule` is non-zero, which only the `bar` style produces), and it is deliberately NOT in the pinned parity table, which carries geometry and timing only.
- `centred: bool` - whether the anchor is a centre one, which is the only thing the blit needs in order to decide whether to re-centre its own measured run.

## lay_one

```rust
pub fn lay_one(item: &TextItem, accent: [u8; 3], ow: u32, oh: u32, t_ms: u32) -> Option<LaidText>
```

Lay one text item out at one instant on the OUTPUT clock, or `None` if it should not be drawn at all.

### Inputs

- `item: &TextItem` - the document's item, already remapped onto the output clock by `remap_doc`, so `t_ms` and `item.start_ms` are in the same time base and an item inside a cut is already gone.
- `accent: [u8; 3]` - `Settings.ui.accent`, RGB. Read by the renderer and passed in (ADDED-4), never stored on the item.
- `ow: u32`, `oh: u32` - the OUTPUT frame size. Every length here is a fraction of one of them, which is what makes a project render the same at any export size.
- `t_ms: u32` - the instant, on the output clock (`FramePose::out_t`).

### The contract, in full

The spec leaves five things open. They are ruled here, and BOTH sides implement exactly this.

**1. When there is nothing to draw.** Outside the span (`t < start_ms` or `t >= end_ms`) the answer is `None`. So is a text whose trimmed main AND trimmed sub are both empty: nothing at all, not a bare plate. A user who clears the text of a plated item should see the plate go with it, not a floating black rectangle.

**2. The two ramps and which one governs.**

```
p     = clamp01((t - start) / max(1, in_ms))
q     = clamp01((end - t)   / max(1, out_ms))
e_in  = ease(easing, p)
e_out = ease(easing, q)
g     = min(e_in, e_out)
```

`min` rather than a product or a piecewise switch, and that is the interesting choice: when `in_ms + out_ms` is LONGER than the span, the two ramps overlap and `g` simply never reaches 1. The item fades up, turns over below full strength and fades down, which is what a viewer would expect. A product would double-dim the middle of every normal item; a piecewise switch would snap at the crossover.

**The governing animation is `anim_in` when `e_in <= e_out`, and `anim_out` otherwise** - that is, whichever ramp produced the minimum. So an item sliding in and fading out slides while the in-ramp is the binding one and fades once the out-ramp takes over, with the handover at the crossover rather than at an arbitrary midpoint.

`ease` is **`crate::export::camera::ease`**, the M3 evaluator with the `Keys` arm. It is NOT `crate::export::easing::ease`: the two disagree on `Smooth`, which `camera` evaluates as `t^2 * (3 - 2t)` and `easing` as `1 - (1 - t)^3`. `src/editor/timeline/model/layoutTrack.ts::ease` returns `c * c * (3 - 2 * c)`, so the camera one is the twin of the preview's and the only one this file may call. Calling the other would pass every Rust test here and fail the TypeScript table by a visible margin at the middle of every ramp.

**3. What each animation does.**

| governing anim | effect |
|---|---|
| `Fade` | `alpha = g` |
| `Slide` | `alpha = g`, and `shift = (1 - g) * oh * SLIDE_FRAC * dir` |
| `Pop` | `alpha = g`, and `scale = POP_FROM + (1 - POP_FROM) * g` |
| `Typewriter` | `alpha = (p > 0 ? 1 : 0) * min(1, q * 4)`, so it is at FULL strength from its first frame and only fades at the very end |

`dir` is `(0, -1)` for the three Top anchors, `(0, 1)` for the three Bottom ones, `(-1, 0)` for MidLeft, `(1, 0)` for MidRight and `(0, 0)` for MidCenter: an item slides in from the edge it is anchored to, and the centre has no edge to come from, which is why the inspector greys Slide out there.

`reveal` is independent of which ramp governs: it is `floor(p * main_chars)` whenever **`anim_in`** is `Typewriter`, and `usize::MAX` otherwise. It keys on `anim_in` and not on the governing animation because a reveal is a property of the entrance; once the line is fully typed it must stay typed, and the out-ramp must not un-type it.

**4. How `scale` is applied.** By scaling `font_px` BEFORE the block is measured, then anchoring the scaled block the usual way. For a centre anchor that grows the block about its centre; for an edge anchor it pins the edge and grows inwards, which is what an edge anchor means. Nothing downstream re-applies `scale`.

**5. Which side the rule is on.** LEFT for a left or centre anchor, RIGHT for a right anchor, `oh * RULE_W` wide, and it reserves `2 * oh * RULE_W` of `block_w` so the run clears it by one rule width. A rule on the right of a right-anchored lower third reads as the frame's edge; on the left it would float.

### The measurements, in order

1. `font_px = max(oh * size.frac(), 8.0) * scale`, `sub_px = font_px * SUB_RATIO`. The minimum of 8 keeps text visible on a tiny export.
2. `text_w = max(main_chars * font_px * ADV, sub_chars * sub_px * ADV)` - the wider of the two lines.
3. `block_w = text_w + plate padding + 2 * rule width`, `block_h = font_px * LINE_H + (sub ? sub_px * LINE_H : 0) + plate padding`.
4. `bx`, `by` from `place(anchor_frac, span, block, margin)`, plus `item.offset` scaled by the frame (so a nudge is a fraction of the frame, not pixels) plus `shift`.
5. The text column starts at `bx + plate_pad/2 + (rule on the left ? 2 * rule_w : 0)`; the main baseline is one `font_px` below the text top and the sub baseline one `LINE_H` plus one `sub_px` below it.

### Returns

`Some(LaidText)`, or `None` for the two cases in point 1.

### Behaviors

- `the_forty_eight_case_layout_table_is_pinned` - 48 cases across four kinds, both sub states, nine anchors, two aspects, four animation pairs and five instants, each flattened to twenty numbers and compared to a GENERATED table to 1e-3. The table is a text block rather than a nested array for three reasons: rustfmt cannot reflow it, it fits the test file's budget, and `src/editor/stage/text/textPreview.test.ts` holds the same 48 lines byte for byte. Regenerate with `cargo test --lib print_text_parity_table -- --ignored --nocapture`, which is the `#[ignore]`d test at the bottom of the file; never hand-write a row.
- `an_empty_text_lays_out_to_nothing_rather_than_a_bare_plate` - a whitespace-only main with no sub, and with a whitespace-only sub, both give `None` even in the `plate` style.
- `typewriter_reveals_on_a_fixed_cadence` - a 40-character line over a 1200 ms in-ramp reveals 0, 10, 20 and 40 characters at 0, 300, 600 and 1200 ms, and still 40 at 3000 ms.
- `outside_the_span_there_is_nothing_at_all` - `None` one millisecond before the start and exactly at the end (the span is half-open), `Some` in the middle.
- `a_long_in_and_out_never_reaches_full_alpha_and_never_inverts` - 4000 ms in plus 4000 ms out over a 4000 ms span: alpha stays inside 0..1, its peak is strictly below 1, and it comes back down rather than snapping. This is the `min` rule measured rather than asserted.
- `texts_at_keeps_array_order_and_drops_the_ones_that_are_not_live`.

## texts_at

```rust
pub fn texts_at(items: &[TextItem], accent: [u8; 3], ow: u32, oh: u32, t_ms: u32) -> Vec<LaidText>
```

`lay_one` over a slice, keeping only the items that are live at `t_ms`.

**Draw order is ARRAY ORDER, which is creation order.** There is deliberately no `layer` field on a text item (spec 5.1): unlike zooms and spotlights, which compete for the same screen and needed the priority layering shipped in 2026-09-14, text items are placed by the user at nine anchors and overlapping two of them is a mistake the user can see and fix by moving one. Adding a layer would be a control that exists to resolve a problem the feature does not have. The timeline lane stacks overlapping items onto rows for LEGIBILITY (`layoutRegions`), and those rows are a display device with no effect on paint order.

### Returns

A `Vec<LaidText>`, possibly empty, in the input's order with the non-live items removed.
