# src-tauri/src/export/fx/caption/captionlayout.rs

Where a spoken caption sits, and nothing else: no font, no frame, no I/O. This is one half of the M5 parity contract (ADDED-5). `ab_glyph` and a browser's text rasterizer cannot agree byte for byte, so the export and the preview do NOT try to match pixels. Instead they share this pure function, which derives the whole geometry - line breaks, pill box, baselines, highlight index, fade - from CHARACTER COUNTS and a fixed advance constant. Each side then centres its own measured glyph run inside the pill these numbers produce.

`src/editor/stage/fx/captionPreview.ts` is the mirror, character for character, and `captionlayout_tests.rs` + `captionPreview.test.ts` pin the SAME two worked cases (1920x1080 bottom/medium, and 1280x720 top/small/two-line) plus the SAME 20-row `PARITY` animation fixture, so a change to one side without the other fails a test rather than drifting silently.

The alternative - rendering a PNG per frame in the backend, the way the spotlight overlay does - was rejected because it makes captions lag by a full IPC round trip during playback, which is the one thing a caption must never do.

## ADV

```rust
pub const ADV: f32 = 0.52;
```

Average glyph advance as a fraction of the font size. NOT a measurement of Inter: it is a constant BOTH sides use, so the pill box and the line breaks are identical in the export and the preview even though the two rasterizers differ. 0.52 is a touch wider than Inter SemiBold's real average, which errs toward a pill with a little air rather than one that clips.

## LINE_H

```rust
pub const LINE_H: f32 = 1.32;
```

Line height as a multiple of the font size. Used both for the pill's height and for the gap between the two baselines of a wrapped caption.

## PAD_X

```rust
pub const PAD_X: f32 = 0.60;
```

Horizontal pill padding on EACH side, as a multiple of the font size (so a pill is `2 * PAD_X * font_px` wider than its text box).

## PAD_Y

```rust
pub const PAD_Y: f32 = 0.34;
```

Vertical pill padding, top and bottom, as a multiple of the font size.

## MARGIN

```rust
pub const MARGIN: f32 = 0.075;
```

Distance from the frame edge to the pill, as a fraction of OUTPUT HEIGHT - height on both axes, so the band sits the same distance down a 16:9 frame and a 9:16 one.

## RISE_LINES

```rust
pub const RISE_LINES: f32 = 0.5;
```

How far a `rise` caption starts BELOW its resting place, in line heights. Half a line is enough to read as a lift without ever pushing the pill off the bottom margin of the frame.

## POP_FROM

```rust
pub const POP_FROM: f32 = 0.92;
```

The font scale a `pop` caption enters at, easing to 1.0. 8% is the largest step that still reads as emphasis rather than as a zoom; the pill is re-laid out around the scaled font, so it grows with the text instead of holding a hole.

**Not a constant any more:** the fade length. It was `FADE_MS = 120.0`; it is now `CaptionStyle::animation_ms`, whose default is 120, so a project that never touched it fades exactly as before.

## MAX_CHARS

```rust
pub const MAX_CHARS: usize = 42;
```

A caption line's character budget (binding decision 3).

## MAX_LINES

```rust
pub const MAX_LINES: usize = 2;
```

How many lines one caption may hold. `layout` truncates past this rather than growing the pill.

## LaidCaption

```rust
pub struct LaidCaption {
    pub lines: Vec<String>, pub font_px: f32, pub line_h: f32,
    pub pill: [f32; 4], pub baselines: Vec<f32>, pub hi: Option<usize>, pub alpha: f32,
    pub rise: f32, pub scale: f32, pub words_shown: Option<usize>, pub word_alpha: f32,
}
```

One caption's geometry at one instant, in OUTPUT pixels.

- `lines` - the wrapped text, at most `MAX_LINES`. Empty for a caption whose text is blank.
- `font_px` - `max(oh * style.height_frac(), 8) * scale`, floored at 8 px so a tiny preview frame still gets legible-sized glyphs rather than sub-pixel ones.
- `line_h` - `font_px * LINE_H`, carried so the draw does not recompute it.
- `pill` - `[x, y, w, h]` of the background pill. It is the TEXT'S BOX whether or not the pill is painted: the glyph run is centred inside it either way. The `rise` offset is ALREADY folded into `y`.
- `baselines` - one per line, in output px, `rise` folded in.
- `hi` - index into the caption's own `words`, or `None` when the highlight is off, there are no word timings, or no word has started yet.
- `alpha` - the caption's own opacity, 0..1.
- `rise` - the downward offset still to be eased away, in px. 0 for every animation but `rise`, and 0 there once the entry is over and on the way out. Reported as well as applied, so a test can pin it.
- `scale` - the font scale the layout was taken at. 0.92..1.0 during a `pop` entry, 1.0 everywhere else.
- `words_shown` - how many words the draw may paint, or `None` to paint the whole caption. `Some(_)` only for `words` with real word timings, which is what makes the fallback to `fade` a one-value decision for the draw.
- `word_alpha` - the newest revealed word's own fade-in, 0..1; 1 for every other animation.

**Why the geometry is folded in but the alphas are not:** the draw already multiplies alphas per glyph, whereas a second place computing `py + rise` is a second place that can drift from the preview. Both sides return the number AND apply it.

## ease_out

```rust
pub fn ease_out(p: f32) -> f32
```

`1 - (1 - p)^3` on a `p` clamped to 0..1 - the ONE easing in the caption renderer, spelled identically in `captionPreview.ts`. Every eased quantity (the `rise` offset, the `pop` scale) goes through it; the alpha ramps stay linear, which is what the pre-M5 fade was.

### Behaviors

- `ease_out_is_the_cubic_both_renderers_spell_the_same_way` - 0, 0.875 at the midpoint, 1, and both clamps.

## ramp

```rust
pub fn ramp(dt: f32, ms: f32) -> f32
```

A linear 0..1 ramp `dt / ms`, clamped - and 1 when `ms <= 0`, which is what makes `animation_ms == 0` mean "no ramp at all" rather than a division by zero. Used for the entry ramp, the exit ramp and each word's own fade-in, so all three share one spelling.

## word_span

```rust
pub fn word_span(cap: &Caption, i: usize) -> Option<(usize, usize)>
```

Word `i`'s half-open CHARACTER range inside the caption's joined text, found by summing earlier words' lengths plus one space each. `None` past the end of `words`.

*Why by character offset:* a caption that wrapped onto two lines has no per-line word list, so both the highlight and the `words` reveal have to address the text by offset to survive a wrap. One walk serves both.

### Behaviors

- `word_span_walks_the_joined_text_by_character_offset` - `(0, 5)`, `(6, 11)`, then `None`.

## wrap_lines

```rust
pub fn wrap_lines(text: &str, max: usize) -> Vec<String>
```

Greedy word wrap at `max` characters.

### Inputs

- `text: &str` - the caption's text, already trimmed by the caller. *Why trimmed there:* `layout` owns the trim so the empty-caption case is decided in one place.
- `max: usize` - the character budget per line. *Why a parameter:* the grouper plans lines at `MAX_CHARS`, and tests pin the behaviour at small budgets where the edge cases are visible.

### Returns

The wrapped lines, never containing empty strings. A word longer than `max` takes a line of its own rather than being cut, because a chopped word reads as a typo on screen.

### Notes

- The ASR grouping pass (`asr::group`, task T3) is meant to plan its lines with THIS function rather than writing a second wrap, so the break the grouper planned and the break the renderer performs are the same break. Until that module lands, this is the only wrap in the tree.
- Splitting is on `split_whitespace`, so a run of spaces or a newline inside a caption collapses to one break opportunity.

## caption_at

```rust
pub fn caption_at(caps: &[Caption], t_ms: u32) -> Option<&Caption>
```

The caption covering `t_ms`, end EXCLUSIVE; `None` in a gap.

### Inputs

- `caps: &[Caption]` - the doc's caption track, already on the output clock. *Why output clock:* every region list in `EditDoc` is (ADDED-1), and `remap_doc` moves captions and their words onto it alongside zooms and effects.
- `t_ms: u32` - the instant, on that same clock (`FramePose::out_t` in the export, `tOut` in the preview).

### Returns

The first caption whose span contains the instant. The exclusive end is what keeps two back-to-back captions from both matching on the shared millisecond.

## layout

```rust
pub fn layout(cap: &Caption, style: &CaptionStyle, ow: u32, oh: u32, t_ms: u32) -> LaidCaption
```

Geometry for one caption at one instant. Pure.

### Inputs

- `cap: &Caption` - the caption to lay out. Its `words` drive `hi` only; the drawn text is `cap.text`.
- `style: &CaptionStyle` - position, size rung or `font_pct`, whether the highlight is on, and the `animation` plus its `animation_ms`. *Why the whole struct:* the panel edits these as one group, and passing the group keeps this signature stable as the look grows - which is exactly what M5's look pass proved, since not one caller changed when eight fields arrived.
- `ow: u32`, `oh: u32` - the output frame. *Why both when only height sets the font:* width centres the pill.
- `t_ms: u32` - the instant, on the output clock. Drives `alpha`, `hi`, `rise`, `scale` and `words_shown`.

### Returns

A `LaidCaption`. An empty caption returns empty `lines`, an all-zero `pill` and no baselines, so a blank line on the track draws nothing at all rather than a bare pill floating over the picture.

### Implementation

1. `p_in = ramp(t - start, animation_ms)` - the entry progress every animation is driven from.
2. `alpha` - 0 outside `[start, end)` for EVERY kind; then 1 for `None`, else `min(p_in, ramp(end - t, animation_ms))`. Linear, as the pre-M5 fade was.
3. `scale = Pop ? POP_FROM + (1 - POP_FROM) * ease_out(p_in) : 1`; `font_px = max(oh * style.height_frac(), 8) * scale`; `line_h = font_px * LINE_H`.
4. `rise = Rise ? line_h * RISE_LINES * (1 - ease_out(p_in)) : 0` - entry only, so the exit is a plain fade with the caption sitting still.
5. `hi` = the LAST word whose `start_ms <= t_ms`, or `None` when `style.highlight` is off or no word has started. *Why the last started rather than the one strictly containing `t`:* a pause between two words would otherwise blink the highlight off mid-sentence.
6. `reveal` - for `Words` with word timings, `words_shown` = how many words have started and `word_alpha` = that newest word's own `ramp`; `(None, 1)` otherwise, which is the fallback to `fade`.
7. `lines = wrap_lines(text.trim(), MAX_CHARS)`, truncated to `MAX_LINES`. Return the zeroed layout if that is empty - `alpha`, `rise`, `scale` and the reveal are already filled in, so even a blank caption reports its animation state.
8. `pill w = widest_line_chars * font_px * ADV + 2 * font_px * PAD_X`; `pill h = lines * line_h + 2 * font_px * PAD_Y`.
9. `pill x = (ow - pill_w) / 2`; `pill y = rise + (Bottom ? oh - oh * MARGIN - pill_h : oh * MARGIN)`. Under a `pop` the pill's BOTTOM stays pinned at the margin, so it grows upward rather than drifting.
10. `baseline i = pill_y + font_px * PAD_Y + line_h * i + font_px`.

### Behaviors

- `case_a_bottom_medium_one_line_at_1920x1080` - the parity table: font 41.04, line height 54.1728, pill `[818.0016, 916.92, 283.9968, 82.08]`, baseline 971.9136, `hi = Some(1)`, alpha 1. UNCHANGED by the M5 look pass, which is the point: the defaults reproduce the old render exactly.
- `case_b_top_small_two_lines_mid_fade_at_1280x720` - font 21.6, line height 28.512, pill `[408.016, 54, 463.968, 71.712]`, baselines 82.944 and 111.456, `hi = None`, alpha 0.5. Also unchanged.
- `every_animation_kind_is_pinned_at_entry_steady_exit_and_past_the_end` - THE PARITY FIXTURE. One 20-row `PARITY` table (five kinds x four instants: 1060 entry midpoint, 2000 steady, 2940 exit midpoint, 3100 past the end) pinning `alpha`, `rise`, `scale`, `words_shown` and `word_alpha`, plus the `font_px` and `pill y` those imply. The identical 20 rows are the `PARITY` table in `captionPreview.test.ts`, so either renderer drifting turns a test red on both sides.
- `the_rise_offset_moves_the_pill_and_its_baselines_together` - 3.3858 px on both at the entry midpoint, so the text never floats out of its pill.
- `a_pop_relays_the_pill_out_around_the_scaled_font` - at `p_in = 0`, the font, the pill width and the pill height are all exactly 0.92 of their resting values.
- `words_falls_back_to_the_fade_when_the_caption_has_no_word_timings` - `words_shown` is `None` and alpha is the plain fade.
- `a_zero_length_animation_cuts_in_at_full_alpha_with_no_offset_or_scale` - `animation_ms = 0` for all four ramped kinds.
- `a_fine_font_percent_overrides_the_size_rung_and_is_clamped` - 0 keeps the rung, 5% is 54 px at 1080, and 0.5% / 40% clamp to 1.5% / 8%.
- `the_caption_fades_out_symmetrically_and_is_gone_outside_its_span` - 0.5 at 60 ms from either end, 0 exactly at both ends.
- `caption_at_picks_the_one_covering_the_instant_and_nothing_at_a_gap` - the exclusive end, the gap, and the empty track.
- `the_highlight_index_tracks_the_word_being_spoken_and_holds_the_last_one_through_a_pause` - including `None` before the first word starts.
- `an_empty_caption_lays_out_to_nothing_rather_than_a_bare_pill`.
- `wrap_lines_breaks_on_words_and_never_drops_one`.

### Private helpers

#### reveal

`fn reveal(cap: &Caption, anim: CaptionAnim, t: f32, ms: f32, t_ms: u32) -> (Option<usize>, f32)` - the `words` animation's two numbers, or `(None, 1.0)` for every other kind AND for a `words` caption with no word timings. *Why the fallback lives here rather than in the draw:* a caption whose ASR produced no per-word timings is common (an edited caption drops its `words`, see `captionEdit`), and deciding it once means neither renderer can forget to fall back.

### Used by

- `src-tauri/src/export/fx/caption/captiondraw.rs` - the only Rust caller; `overlay` asks for the geometry and then paints it.
- `src/editor/stage/fx/captionPreview.ts` - the mirror, not a caller: it re-implements this arithmetic and its test pins the same numbers.
