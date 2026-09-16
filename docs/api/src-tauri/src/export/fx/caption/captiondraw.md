# src-tauri/src/export/fx/caption/captiondraw.rs

The spoken-caption blit: a scrim pill, a centred glyph run, and a highlight on the word being spoken. Every number it positions against comes from `captionlayout`, which the preview mirrors, so this file only decides how the pixels land - never where the caption sits, and never how far into an animation the caption is.

It is a CPU `ab_glyph` blit, and it draws with the tree's one embedded face and its one BGRA blend, `export/fx/glyph.rs`'s `font()` and `put` (which `hotkeycap.rs` used to own and which moved out when a third overlay, the animated text items, needed them too). What it deliberately does NOT share is `glyph::draw_run`: the word reveal and the spoken-word highlight give individual glyphs different colours and different alphas inside one line, and a single-colour single-alpha run cannot express that, so `draw_line` keeps its own loop over `put`. It is called from `FrameRenderer::composite_at` directly rather than through `fx_state::render`, because that function already carries eighteen arguments and a `#[allow(clippy::too_many_arguments)]`. Since every output format composites through `composite_at`, GIF export keeps captions for free.

Ordering: after the FX pass, before the cursor. A caption sits over the picture and its effects, but never over the pointer.

## overlay

```rust
pub fn overlay(out: &mut [u8], ow: u32, oh: u32, caps: &[Caption], style: &CaptionStyle,
               accent: [u8; 3], t_ms: u32)
```

Draw the caption covering `t_ms` onto the composited BGRA frame.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame. *Why mutable:* the caption is blended in place over the full composite.
- `ow: u32`, `oh: u32` - output frame dimensions, forwarded to `captionlayout::layout`.
- `caps: &[Caption]` - the caption track on the OUTPUT clock (`FrameRenderer::captions`, refreshed by `reload_edit`).
- `style: &CaptionStyle` - `Settings::captions`: enabled, position, size, pill on/off, highlight on/off, and the whole M5 look - `text_color`, `highlight_color`, `pill_color`, `pill_alpha`. *Why the doc settings and not a render-time flag:* the Captions panel edits these and the export must read exactly what the preview drew.
- `accent: [u8; 3]` - `Settings::ui::accent`, RGB, used ONLY when `style.highlight_color` is `None`. *Why passed in rather than read here:* there is exactly one accent in the doc and the renderer owns reading it (ADDED-4), so this function stays free of settings lookups. *Why still passed once the style can carry its own colour:* `None` is the default and means "follow the interface accent", so a project that never picked a caption colour re-themes with the app.
- `t_ms: u32` - the instant on the output clock (`FramePose::out_t`).

### Returns

`()`. A no-op when `style.enabled` is false, when `caption_at` finds nothing, when the animation has not started, when the caption's text is blank, or when the embedded font fails to parse.

### Implementation

1. Early-out on `!style.enabled`, then on `caption_at(caps, t_ms)` being `None`.
2. `layout` the caption; early-out on `alpha <= 0` or no lines.
3. When `style.pill`, fill the rounded pill through `scrim` in `style.pill_color` at `alpha * pill_alpha / 100` (62 reproduces the old hard-coded `SCRIM`).
4. Build the `Pen`: the two colours (`highlight_color.unwrap_or(accent)` for the lit word), the lit word's half-open CHARACTER range (`word_span`), the reveal cut (`cut_at`) and `word_alpha`. Ranges are character offsets, so both the highlight and the reveal survive a wrap onto the second line.
5. Draw each line through `draw_line`, advancing the running character base by `line.chars().count() + 1` - the `+ 1` being the space `wrap_lines` swallowed at the break.

### Behaviors

- `a_caption_paints_in_the_bottom_band_and_leaves_the_top_alone`.
- `top_position_moves_the_paint_to_the_top_band`.
- `nothing_is_drawn_when_disabled_between_captions_or_with_an_empty_track` - all three no-op paths leave the buffer byte-identical.
- `turning_the_pill_off_paints_strictly_fewer_pixels_than_leaving_it_on`.
- `the_highlighted_word_carries_the_accent` - with a saturated blue accent and `highlight_color: None`, some pixel comes back blue-dominant.
- `the_style_can_override_the_text_and_highlight_colours_and_then_the_accent_is_unused` - green body, red highlight, and NOT ONE blue pixel though a blue accent was passed.
- `the_pill_takes_its_own_colour_and_its_own_alpha` - an opaque white pill shows white; `pill_alpha: 0` paints nothing at all.
- `the_words_animation_reveals_one_word_at_a_time_inside_a_pill_that_never_moves` - ink grows from the first word to the second while the scrim's pixel count is identical, which is the "the pill does not jump" guarantee measured rather than asserted.
- `the_none_animation_is_already_at_full_strength_on_the_caption_s_first_frame` - at `t = start`, `fade` paints zero pixels and `none` paints many.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::composite_at`, one call, after `fx_state::render` and before the cursor draw.

### Private helpers (all called only by `overlay`)

#### Pen

`struct Pen { px, alpha, text, lit, hi, cut, word_alpha }` - everything a line needs that is not its own text or baseline, bundled so `draw_line` keeps a readable arity as the look grows. `hi` and `cut` are half-open CHARACTER ranges over the caption's joined text, not per-line indices.

*Why `hi_range` is gone:* it became `captionlayout::word_span`, because the `words` reveal needs the same walk and the two must agree exactly.

#### cut_at

`fn cut_at(cap: &Caption, shown: usize) -> (usize, usize)` - the reveal boundary for `words`: the newest revealed word's character range, or `(0, 0)` when no word has started (which hides the whole caption while the pill is already at full size). Characters before the range paint at full `alpha`, characters inside it at `alpha * word_alpha`, characters after it not at all.

#### scrim

`fn scrim(out: &mut [u8], ow: u32, oh: u32, r: [f32; 4], a: f32, col: [u8; 3])` - fills the rounded pill with `col` at `a`, coverage-antialiased at the corners by one signed-distance expression that covers all four (distance outside the corner-inset box, zero anywhere in the straight part).

It also raises each covered pixel's ALPHA byte. This is deliberate and worth stating: the frame is opaque BGRA and that byte is otherwise unread, while the default black pill over an already-black pixel moves no colour byte at all - so the alpha byte is what records that a scrim was laid down there. It is what the pill-on/pill-off and pill-never-jumps tests read, and it is the safe value for a muxer that does look at alpha.

#### draw_line

`fn draw_line(...)` - one line of glyphs, centred on the pill's centre x and sitting on its baseline. `ab_glyph`'s own measured advances place the run, so the export uses REAL Inter metrics inside the character-count pill `captionlayout` handed it - that split is the whole of ADDED-5. Characters inside the highlight range take `pen.lit`, the rest `pen.text`; both get the same 1 px dark shadow at 0.6 coverage that `glyph::draw_run` lays down, which is what keeps text legible when the pill is off.

Under a `words` reveal each glyph's alpha is decided by `pen.cut` before it is rasterised, and an unrevealed glyph STILL ADVANCES `x`. That is the invariant worth keeping: the run is centred on the whole caption's width from the first frame, so a word appearing never reflows the words already on screen.
