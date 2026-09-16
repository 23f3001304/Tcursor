# src-tauri/src/export/fx/text/textdraw.rs

The animated text blit: the plate, the rule and the two glyph runs, painted onto the composited BGRA frame from the boxes `textlayout` produced.

Everything about WHERE a text item sits and HOW FAR into its animation it is comes from `textlayout::texts_at`; this file only decides how the pixels land. That is the same split `captiondraw` has against `captionlayout`, and it is what lets the preview mirror the layout without mirroring the rasteriser.

`src/editor/stage/text/textDraw.ts` is its twin on the 2D canvas, drawing the same four things in the same order.

## overlay

```rust
pub fn overlay(out: &mut [u8], ow: u32, oh: u32, items: &[TextItem], accent: [u8; 3], t_ms: u32)
```

Draw every text item that is live at `t_ms`.

### Where it is called from

`FrameRenderer::fx_pass`, between `fx_state::render` and `captiondraw::overlay`. Both halves of that position are rulings, not conveniences:

- **After the FX pass**, so a colour grade, a spotlight scrim or a video effect does not tint a title whose colour the user explicitly chose. Effects belong to the picture; text is laid on top of it.
- **Before the captions**, because captions are drawn last and nothing may cover them (spec 4, fit point 1). A text item placed at the bottom centre therefore COLLIDES with a caption rather than winning; the Text inspector hints that collision to the user and nothing in the renderer moves anything to avoid it, because silently relocating something the user positioned is worse than letting them see the overlap.

The cursor is blitted after the whole seam returns (`FrameRenderer::composite_at`), so it stays on top of both.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame. *Why mutable:* everything here is blended in place.
- `ow: u32`, `oh: u32` - the output frame size, which is also what every length in the layout is a fraction of.
- `items: &[TextItem]` - `FrameRenderer::texts`, already on the OUTPUT clock (`remap_doc` moved them there), refreshed by `reload_edit`.
- `accent: [u8; 3]` - `Settings.ui.accent`, RGB, forwarded to `texts_at` and resolved there onto each laid item's `fill` and `rule_rgb`. One accent in the document, read by the renderer (ADDED-4).
- `t_ms: u32` - the instant on the output clock (`FramePose::out_t`).

### Returns

`()`. Four no-op paths, all byte-identical: an empty item list, an instant at which nothing is live, a font that will not parse, and any individual item whose `alpha` has reached zero.

### The paint order, per item

1. **The plate** - a rounded scrim at `plate_alpha * alpha`, radius `font_px * PLATE_RADIUS`.
2. **The rule** - the `bar` style's bar, in `rule_rgb` (the document accent) at the item's full alpha, square-cornered.
3. **The main run** - the revealed characters only, at the item's alpha.
4. **The sub run** - at `alpha * 0.82`.

The runs go last so the glyphs sit over their own background rather than being dimmed by it.

### Centring, and why each side measures its own run

`run_x` is the ADDED-5 idiom. `textlayout` produced a BOX from character counts at the fixed advance `ADV`, which both Rust and TypeScript can compute identically without sharing font metrics. Inside that box, each side centres its OWN truly measured run: here `glyph::run_width` on real Inter advances, in the preview `ctx.measureText`. So the two sides agree exactly on the box, each is exactly correct about its own glyphs, and neither has to know anything about the other's text stack.

A non-centred anchor skips all of it and starts at the box's left edge, which is what a left or right anchor means.

### The sub line's 0.82

The second line draws at `0.82` of the item's alpha. It is the one look constant this file owns rather than inheriting from the layout, and it is here rather than beside `SUB_RATIO` because it is a rendering decision, not a measurement: the sub is already smaller, and dropping it slightly in weight as well is what makes a lower third read as a name with a role under it rather than as two lines of equal importance. `src/editor/stage/text/textDraw.ts` names the same number `SUB_ALPHA`.

### Behaviors

- `a_title_paints_in_the_band_its_anchor_names_and_leaves_the_others_alone` - all nine anchors, each checked to paint into its own third of a 3x3 grid and to leave the diagonally opposite third completely untouched.
- `alpha_zero_is_a_byte_identical_no_op` - an item whose span has passed leaves a patterned buffer byte for byte identical, plate style included.
- `the_plate_style_paints_a_scrim_and_the_bar_style_paints_a_rule_in_the_accent` - the plate darkens a white frame; the bar leaves a red-dominant pixel on a black one, which is the accent and not the white glyph fill.
- `a_typewriter_draws_only_the_revealed_characters` - strictly more lit pixels later in the reveal than earlier.
- `the_pass_order_is_text_over_the_effects_and_under_the_captions` - a source-order assertion over `render/fx_step.rs`, added in the task that wired the call up. It is a blunt instrument on purpose: the alternative is an end-to-end frame render inside a unit test, and this fails loudly the moment someone reorders the seam, which is the whole point of the ruling.

## PLATE_RADIUS

```rust
const PLATE_RADIUS: f32 = 0.35;
```

The plate's corner radius as a fraction of the MAIN font size, clamped in `fill_rect` to half the plate's width and half its height so a short plate cannot invert its own corners.

A fraction of the font size rather than of the plate: the radius then reads as a property of the type, so a two-line plate and a one-line plate have the same corner and look like the same component at different heights.

## rrect_sd

```rust
fn rrect_sd(x: f32, y: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32
```

The signed distance to a rounded rectangle: negative inside, zero on the edge, positive outside. `fill_rect` turns it into coverage as `clamp01(0.5 - sd)`, which antialiases the corners over roughly one pixel.

**This copy is deliberately temporary.** Track 2a of the same batch publishes `crate::export::fx::mask::rrect_sd` with a character-for-character identical body, and the two tracks ran in parallel worktrees where neither could see the other's module. The Shared file ledger's last row is the contract: at integration the controller DELETES this private copy, imports the mask module's, and re-runs `cargo test --lib textdraw`. Every test in this file passing unchanged is the proof the two bodies had not drifted. If one fails instead, the bodies HAD drifted and that is a bug to open, not to paper over.
