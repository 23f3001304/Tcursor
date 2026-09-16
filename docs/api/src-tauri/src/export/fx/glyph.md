# src-tauri/src/export/fx/glyph.rs

The one glyph blit in the tree: the embedded face, the run measurement, the run rasteriser and the BGRA alpha blend that every text overlay composites through.

It exists because three overlays now rasterise `ab_glyph` outlines - the hotkey chord caption, the spoken-caption track and the animated text items of Batch 2 - and three copies of the same outline loop is how this file set goes over its line budget. `hotkeycap.rs` owned `FONT` and `put` as `pub(crate)` for exactly this reason (M5 borrowed them for `captiondraw.rs`); this module is that sharing made explicit before the third caller arrives, and it is spec 4's fit point 2.

The move is behaviour free by construction: `draw_caption`'s loop became `draw_run` byte for byte, and both importers' pre-existing tests pass unchanged. That is what makes it safe.

`captiondraw.rs` keeps its own per-glyph loop rather than calling `draw_run`, and deliberately: the word reveal and the spoken-word highlight give individual glyphs different colours and different alphas inside one line, which a single-colour single-alpha run cannot express. Only the font handle and the pixel blend are shared with it.

## FONT

```rust
pub const FONT: &[u8];
```

The bytes of `src-tauri/assets/fonts/Inter-SemiBold.ttf`, embedded at compile time.

This is the ONLY font in the tree and nothing below adds another (spec 8 puts custom fonts out of scope). Every overlay that draws letters onto a frame draws them in this face, so the binary carries one copy of it and no overlay can drift from another typographically. The preview's own canvas painters ask for `"Inter Variable", system-ui, sans-serif` in CSS, which is the same family at the same weight, and the two sides are reconciled not by sharing metrics but by each centring its own measured run inside a box the shared layout produced (the ADDED-5 idiom).

## font

```rust
pub fn font() -> Option<FontRef<'static>>
```

Parse `FONT` into a face, or `None` if the embedded bytes will not parse.

### Returns

`Some(FontRef)` in every build that ships the asset. `None` is the failure every caller treats as "draw nothing at all": a frame without a caption is far better than a panicking export, so each overlay early-returns rather than unwrapping.

*Why it re-parses per call rather than caching:* `FontRef::try_from_slice` only validates and indexes the table directory over borrowed bytes, so it costs microseconds and allocates nothing. A `OnceLock` would buy nothing measurable at one call per overlay per frame and would add a global.

### Behaviors

- `the_bundled_face_loads` - the shipped asset parses.

## run_width

```rust
pub fn run_width(font: &FontRef, px: f32, text: &str) -> f32
```

The advance width of `text` set at `px` pixels, in pixels: the sum of every character's horizontal advance in that face at that scale.

This is REAL Inter metrics, not the character-count approximation `captionlayout::ADV` and `text::textlayout` use to place a box. The split is deliberate: a shared layout function decides where the box goes from character counts, which both Rust and TypeScript can compute identically without shipping font metrics to the browser, and then each side centres its own truly measured run inside that box.

### Inputs

- `font: &FontRef` - the face, from `font()`.
- `px: f32` - the pixel size the run will be drawn at, which is also the scale the advances are measured at.
- `text: &str` - the run. Measured by `chars()`, so it is codepoints and not grapheme clusters; no shaping, no kerning pairs, no ligatures.

### Returns

The width in pixels. Exactly `0.0` for an empty string, which is the value callers compare against when deciding whether there is anything to centre.

### Behaviors

- `run_width_grows_with_the_text_and_with_the_size` - a longer string is wider, doubling `px` more than doubles nothing and at least 1.9x the width, and the empty string measures zero.

## draw_run

```rust
pub fn draw_run(out: &mut [u8], ow: u32, oh: u32, font: &FontRef, px: f32, x: f32, y: f32,
                text: &str, rgb: [u8; 3], alpha: f32, shadow: bool)
```

Rasterise `text` onto the BGRA frame in one colour at one alpha, with an optional drop shadow.

### The convention

`x` is the LEFT EDGE of the run's first advance, and `y` is the BASELINE, not the top of the glyph box. Ascenders rise above `y`, descenders fall below it. A caller that wants a centred run computes its own left edge from `run_width`; a caller that has a box from a layout function positions the baseline itself. Nothing here knows about anchors, boxes or centring.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame. *Why mutable:* coverage samples are blended in place over whatever is already there.
- `ow: u32`, `oh: u32` - frame dimensions, used only for the bounds check inside `put`; a run that starts off-screen simply paints its visible part.
- `font: &FontRef` - the face, from `font()`.
- `px: f32` - the pixel size.
- `x: f32` - the left edge (see the convention above).
- `y: f32` - the baseline (see the convention above).
- `text: &str` - the run, walked by `chars()`.
- `rgb: [u8; 3]` - the fill colour in RGB order. `put` is what reverses it into the frame's BGRA byte order, so callers never hand-swap channels.
- `alpha: f32` - the whole run's opacity, multiplied into every coverage sample. This is how a fade costs no new layout.
- `shadow: bool` - whether to lay a dark shadow under the run.

### Returns

`()`. A no-op when `text` is empty or `alpha <= 0.0`, checked before the face is touched.

### The shadow

When `shadow` is true, each coverage sample is painted TWICE: first black `[0, 0, 0]` at one pixel down and one pixel right (`bx + 1, by + 1`) at `cov * alpha * 0.6`, then the fill at `(bx, by)` at `cov * alpha`.

*Why 0.6 of the run's alpha:* strong enough to separate a white glyph from a light background, weak enough not to read as a second darker glyph; and scaling it by the run's own alpha means the shadow fades with the text rather than outliving it. *Why one pixel and not a blur:* it is a legibility device, not a look, and one pixel is resolution independent enough at every size the app draws at while costing one extra blend per sample.

It is optional because a glyph drawn on top of an opaque plate does not need it and would only muddy the plate's edge, which is exactly what the `plate` text style asks for.

### Behaviors

- `draw_run_paints_at_the_left_edge_and_the_baseline` - "IJ" at `px = 40`, `x = 20`, `y = 60` lights pixels whose leftmost column is near x=20 and whose lowest row does not pass the baseline, since neither glyph descends.
- `a_shadow_darkens_one_pixel_down_and_right_and_is_optional` - the same run with and without the shadow differs, and differs by getting DARKER. The test paints on a mid-grey canvas rather than a black one on purpose: a black shadow blended over black moves no byte, so a zeroed buffer cannot observe the thing the test is named for.

## put

```rust
pub fn put(out: &mut [u8], ow: u32, oh: u32, x: i32, y: i32, c: [u8; 3], a: f32)
```

Blend one colour at one alpha into one pixel of the BGRA frame. The single text blend in the tree: every overlay that writes a letter or a scrim writes through here, so none of them can drift in how it composites.

### The channel order

`c` is given as RGB and the frame is BGRA, so the mapping is deliberately crossed: `c[2]` (blue) lands at byte `i`, `c[1]` (green) at `i + 1`, `c[0]` (red) at `i + 2`. Byte `i + 3` (alpha) is left ALONE - the frame is opaque and that byte is otherwise unread, which is what lets `captiondraw::scrim` use it as its own private record that a pill was laid down.

### Inputs

- `out: &mut [u8]`, `ow: u32`, `oh: u32` - the frame and its dimensions.
- `x: i32`, `y: i32` - the pixel. Signed, because glyph bounding boxes routinely start left of or above the frame and a caller should not have to pre-clip.
- `c: [u8; 3]` - the colour in RGB.
- `a: f32` - the coverage, clamped to 0..1 before use.

### Returns

`()`. Three no-ops, all silent: any coordinate outside the frame, an `a` that clamps to zero or below, and (by arithmetic) a colour equal to what is already there.

### Behaviors

- `put_blends_over_bgra_and_ignores_out_of_bounds` - red `[255, 0, 0]` at full alpha lands as `[0, 0, 255]` in the frame's bytes, which is the channel cross above; negative coordinates, coordinates past the frame and `a = 0.0` all leave the buffer untouched.

### Used by

- `src-tauri/src/export/fx/click/hotkeycap.rs` - through `draw_run`, for the hotkey chord caption.
- `src-tauri/src/export/fx/caption/captiondraw.rs` - directly, per glyph, because the word reveal and the highlight vary colour and alpha inside one line; and for the pill scrim.
- `src-tauri/src/export/fx/text/textdraw.rs` - through `draw_run` for the two runs, and directly for the plate and the rule.
