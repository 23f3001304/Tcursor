# src-tauri/src/export/cursor/captured.rs

The REAL OS cursor, composited from the layer the recorder captured (`events::track::cursorlayer`). This is what "System" means on any recording made since the capture went cursor-free: the actual bitmap that was on screen, at the raw recorded point, with none of the Enhanced polish - no follow glide, no path idealization, no click bounce, no motion trail.

The three styles now split cleanly:

| Style | Layer present | What draws |
| --- | --- | --- |
| System | yes | this file - the captured bitmap |
| System | no (pre-layer recording, cursor baked in) | nothing (`cursorset::prep` returns `None`) |
| System | no (pre-layer Enhanced/Hidden take) | `cursorset`'s plain-OS arrow |
| Enhanced | either | `cursorset` - the synthetic sprite stack, unchanged |
| Hidden | either | nothing |

## draws_captured

```rust
pub fn draws_captured(style: CursorStyle, has_layer: bool) -> bool
```

Whether this frame draws the captured cursor instead of the synthetic one.

### Inputs

- `style: CursorStyle` - read LIVE from the edit doc, not cached on the renderer, so switching style in the editor takes effect in the warm preview immediately (the same reason `cursorset::draw` re-reads it per frame).
- `has_layer: bool` - a property of the RECORDING (`CapturedCursors::load` succeeded), never of the doc.

### Returns

`true` only for `System` with a layer. That is the single gate: when it is true `composite_at` skips the synthetic path entirely, which is what makes the plain-arrow fallback apply *only* where there is no layer to composite.

## content_scale

```rust
pub fn content_scale(panel: f32, inset_w: f32, sw: u32) -> f32
```

Output pixels per SOURCE pixel for this frame's screen panel - the scale that keeps the captured cursor at its true size **relative to the screen content**.

### Inputs

- `panel: f32` - the screen panel's width against the export's fixed `inset_w` reference, from `cursorset::frame_placement` (0.1..1.0).
- `inset_w: f32` - the full inset region's width in output pixels.
- `sw: u32` - the SOURCE video's width in pixels (`FrameRenderer.sw`, from `probe_dims`).

### Returns

`panel * inset_w / sw`. Since `panel * inset_w` IS the screen panel's own width, this is exactly `screen.rect.w / sw` - the same ratio the compositor draws the screen content at. Writing it through `panel` keeps the factor the synthetic cursor shares visible, and applies the panel shrink exactly ONCE (multiplying `panel` by `screen.rect.w / sw` would square it, so a half-width custom-arrangement panel would quarter the cursor).

*Why width and not height:* `coordmap::to_panel` maps the source into the panel rect on each axis independently, and the synthetic path's `panel` is width-derived too - so width keeps all three consistent. For the aspect-fitted layouts the two ratios agree anyway.

*What it fixed:* before this, the captured bitmap was drawn at its captured pixel size scaled by `panel` alone, so a 4K take exported at 1080p showed a cursor about twice its true on-screen proportion. A take whose source and inset widths match is unaffected (the factor is 1).

*Degenerate `sw`:* clamped to 1, so a zero can never divide.

## CapturedCursors

```rust
pub struct CapturedCursors { sprites: HashMap<u32, CursorSprite>, layer: CursorLayer }
```

The recording's cursor layer, decoded once per `FrameRenderer`: every captured bitmap as a `CursorSprite` (BGRA, so the existing blit can draw it), keyed by layer id, plus the timeline saying which was showing when.

Held as an `Option` field on `FrameRenderer` beside `cprep`, and built in `FrameRenderer::new` for the same reason: it is edit-independent, so a warm preview must not redo it on every doc change.

## CapturedCursors::load

```rust
pub fn load(paths: &ProjectPaths) -> Option<Self>
```

Load `cursor/layer.json` and decode each entry's PNG.

### Returns

`Some` when at least one bitmap decoded; `None` for a recording with no layer, or one whose PNGs are all unreadable. Never an error - a missing layer just means the synthetic path stays in charge.

### Implementation

Each PNG is decoded to RGBA8 (`normalize_to_color8 | ALPHA`, the same transformations `win::sys::brand_icon` uses, so paletted/16-bit/no-alpha inputs all normalize), swapped to BGRA in place, and wrapped in a `CursorSprite` whose `hot` is the entry's pixel hotspot re-expressed as a 0..1 fraction and whose `canvas_h` is the bitmap's own height. Those two choices are what let `cursordraw::draw_cursor` be reused verbatim - see `draw`.

## CapturedCursors::sprite_at

```rust
pub fn sprite_at(&self, t_ms: u32) -> Option<&CursorSprite>
```

The bitmap showing at EVENT time `t_ms` (`pose.ev_t`, the raw-stream clock - not output time), via `CursorLayer::id_at`. `None` before the first sample, or if that id's PNG failed to decode.

## CapturedCursors::draw

```rust
pub fn draw(&self, out: &mut [u8], ow: u32, oh: u32, cur: FramePoint, cam: Camera, screen: &Panel, inset_w: f32, sw: u32, ev_t: u32)
```

Per-frame draw, taking the same arguments as `cursorset::draw` (plus the source width) so `composite_at` can pick between the two without reshaping anything.

### Inputs

- `out`, `ow`, `oh` - the composited frame, BGRA.
- `cur: FramePoint` - the cursor in base/output coordinates. In this mode it is the RAW recorded path: `CursorSettings::plain_os` is true for System-without-a-baked-cursor, so `follow_alpha_at` is 1.0 (no glide) and `idealize_at` is 0.0 (no straightening) - `Cursor::at` returns the interpolated sample verbatim.
- `cam`, `screen`, `inset_w` - projected through `cursorset::frame_placement`, the helper both cursor paths share, so the panel scale and the clip box can never drift apart.
- `sw: u32` - the source video's width, for `content_scale`. The synthetic path does not need it (its sprites are authored against the output canvas); the captured bitmaps are in source pixels, so they do.
- `ev_t: u32` - event time, for `sprite_at`.

### Implementation

`frame_placement` yields the hotspot's on-screen point, the panel factor and the clip box, or `None` once the screen panel is more than half faded (no screen, no cursor). The sprite is then blitted through `cursordraw::draw_cursor` with `size_px = spr.canvas_h * content_scale(panel, inset_w, sw)`, an empty trail and `bounce = 1.0`.

*Why that `size_px`:* `draw_cursor` derives `scale = size_px * bounce / canvas_h`, so asking for `canvas_h * s` pixels of height IS a scale of exactly `s`, and the hotspot fraction set in `load` lands the sprite's top-left at `point - hotspot * s`. At `s == 1` (source and panel the same width) that is the captured bitmap at 1:1, hotspot exactly on the recorded point.

### Behaviors

- `the_sprite_lands_with_its_hotspot_on_the_raw_recorded_point` - at scale 1, top-left at `point - hotspot`, hotspot pixel on the point, nothing outside.
- `a_source_twice_the_output_draws_the_cursor_at_half_size` - the same 4x4 cursor in the same 40px panel spans 4x4 from a 40px source and 2x2 from an 80px one, still hotspot-anchored on the recorded point.
- `content_scale_is_the_source_to_panel_ratio_with_the_panel_shrink_applied_once` - the ratio table, including that a half-width panel halves (not quarters) the result.
- `nothing_is_drawn_before_the_first_sample_or_with_the_panel_faded_out`, `a_recording_with_no_layer_falls_back_to_the_synthetic_arrow`, `only_system_with_a_layer_draws_the_captured_cursor`.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::composite_at`
