# src-tauri/src/export/cursor/cursordraw.rs

CPU rasterizer for the Enhanced synthetic cursor: PNG sprite decoding, click-bounce scale animation, motion trail compositing, and alpha-blitted placement onto a BGRA frame. All drawing is confined to a caller-supplied clip rect so the cursor and trail never spill outside the screen panel. The per-type sprite set and theme inversion live in `cursorset`; this file is the low-level draw layer.

## CursorSprite

```rust
pub struct CursorSprite {
    pub bgra: Vec<u8>,
    pub w: u32,
    pub h: u32,
    pub hot: (f32, f32),
    pub canvas_h: u32,
}
```

Decoded, content-tight cursor sprite ready for blitting.

- `bgra` - raw BGRA pixel data, row-major, `w * h * 4` bytes. *Why BGRA:* all frame buffers in the pipeline are BGRA; storing sprites in the same format avoids per-blit colour-order conversion.
- `w`, `h` - sprite dimensions in pixels after tight-cropping fully-transparent rows and columns. *Why content-tight:* blitting a smaller region is faster, and the hotspot fractions remain correct after cropping because `ffio::decode_cursor` re-bases them.
- `hot` - hotspot as fractions of the content `(w, h)`. *Why fractions, not pixels:* scale-independent; `draw_cursor` multiplies by the scaled dimensions at render time, so the hotspot tracks any output size.
- `canvas_h` - height of the original canvas the sprite was authored for (before content-trim), in pixels. *Why:* `draw_cursor` divides by `canvas_h` when computing the scale factor so that all cursor shapes in a pack maintain their authored proportions relative to each other, rather than each being stretched to the same absolute height.

### Used by

- `src-tauri/src/export/cursor/cursorset.rs` - stores one `CursorSprite` per `CursorType` in `CursorPrep.set`; calls `decode_sprite` at prep time and `apply_enhanced` per frame.

## decode_sprite

```rust
pub fn decode_sprite(png: &[u8], hot: (f32, f32)) -> Option<CursorSprite>
```

Decodes a cursor PNG to a content-tight `CursorSprite` with the hotspot re-based to the cropped content region.

### Inputs

- `png: &[u8]` - raw PNG bytes (compiled in with `include_bytes!`). *Why bytes:* the sprites are embedded in the binary at compile time so no runtime file access is needed.
- `hot: (f32, f32)` - hotspot as `(x_fraction, y_fraction)` of the original full canvas. *Why canvas fractions:* asset authors define hotspots relative to the uncropped canvas; `ffio::decode_cursor` converts them to content-region fractions automatically.

### Returns

`Some(CursorSprite)` on success. `None` if the PNG cannot be decoded (corrupt or missing asset). `cursorset::prep` skips failed non-Arrow types and returns `None` entirely when Arrow fails.

### Implementation

1. Delegates fully to `crate::export::ffio::decode_cursor(png, hot)` which content-crops, re-bases the hotspot, and returns the decoded BGRA data together with `canvas_h`.
2. Wraps the result in `CursorSprite`.

## bounce_scale

```rust
pub fn bounce_scale(click_ms: &[u32], t_ms: u32, enabled: bool, intensity: f32) -> f32
```

Returns a scale multiplier (1.0 normally, dips briefly below 1.0 after each click) that visually acknowledges click events by shrinking the cursor slightly.

### Inputs

- `click_ms: &[u32]` - sorted ascending list of `MouseDown` timestamps in milliseconds. *Why only Down events:* moves and ups do not signal a deliberate click; `cursorset::prep` filters to `EventKind::Down` before storing this list.
- `t_ms: u32` - current frame event-time in milliseconds. *Why:* determines how far past the most recent click we are, driving the linear recovery curve.
- `enabled: bool` - whether click-bounce is active. *Why explicit flag:* allows the user to toggle the feature with no second code path; returns 1.0 immediately when false.
- `intensity: f32` - dip depth, 0..1. *Why:* at the design default of 0.5 the dip is `0.36 * 0.5 = 0.18` (18% shrink at the click instant); at 1.0 it is 36%. Allows user-tunable feedback strength without changing the curve shape.

### Returns

`f32` in `(0.64..=1.0]`. Returns 1.0 when disabled, when `click_ms` is empty, or when the most recent click is more than 180 ms ago (cursor has fully recovered). Otherwise the multiplier drops linearly from `1 - 0.36 * intensity` at `dt = 0` back to 1.0 at `dt = 180 ms`.

### Behaviors

- `bounce_is_identity_when_disabled_or_idle` - returns 1.0 for: no clicks; `enabled = false`; click older than 180 ms.
- `bounce_dips_right_after_a_click` - returns a value strictly below 1.0 within 10 ms of a click at `intensity = 0.5`.
- `higher_intensity_produces_deeper_dip` - intensity 1.0 yields a lower multiplier than intensity 0.3 at the same timestamp.

## draw_cursor

```rust
pub fn draw_cursor(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite,
                   pos: (f32, f32), recent: &[(f32, f32)],
                   size_px: f32, blur: f32, bounce: f32, clip: (i32, i32, i32, i32))
```

Composites the cursor sprite and its motion trail onto `out`, confined to `clip`.

### Inputs

- `out: &mut [u8]` - BGRA frame buffer, `ow * oh * 4` bytes. *Why mutable:* alpha-blending is applied in place over whatever the compositor already wrote.
- `ow`, `oh` - output frame dimensions. *Why:* used for the blit's pixel-index stride computation and for the `blit` bounds clamp.
- `spr: &CursorSprite` - decoded sprite for the active cursor type. *Why passed in:* the caller (`cursorset::draw`) selects the correct type-specific sprite before calling this function, keeping sprite selection and drawing decoupled.
- `pos: (f32, f32)` - hotspot position in output pixels (frame-local, already projected through the camera). *Why float:* sub-pixel precision prevents a 1-pixel jump when the camera is panning between frames.
- `recent: &[(f32, f32)]` - previous hotspot positions for the motion trail, newest first. *Why newest first:* the fade loop divides index by `n`, so index 0 (newest, closest to the cursor) has the highest alpha; older positions fade out.
- `size_px: f32` - target full-canvas height in output pixels (derived from `oh * 0.033 * size * panel` by `apply_enhanced`). *Why canvas height rather than sprite height:* all sprites in a pack are sized relative to the same canvas so a wide resize arrow maintains its authored width-to-height ratio rather than being uniformly stretched to the pointer's height.
- `blur: f32` - trail strength 0..1. *Why:* 0 disables the trail loop entirely via the `if n > 0 && blur > 0.0` guard, avoiding unnecessary blit calls.
- `bounce: f32` - scale multiplier from `bounce_scale` (< 1.0 immediately after a click). *Why passed in:* decouples the animation from the blit; any caller can supply a different multiplier.
- `clip: (i32, i32, i32, i32)` - `(x0, y0, x1, y1)` blit boundary. *Why:* prevents the cursor and trail from spilling onto the webcam panel or the solid background region outside the screen panel.

### Returns

`()`. Modifies `out` in place.

### Implementation

1. Compute `scale = (size_px * bounce / spr.canvas_h).max(0.0001)`. Clamped to a minimum so the blit dimensions cannot become zero.
2. Derive scaled sprite dimensions `(tw, th)` and hotspot pixel offsets `(hx, hy)` from `spr.hot * (tw, th)`. Compute `top_left = (pos.0 - hx, pos.1 - hy)`.
3. If `n > 0 && blur > 0.0`, iterate `recent`: for each entry at index `i`, compute `fade = 1 - i / n` and `alpha = blur * fade * 0.5`, then call `blit` at that position. Older entries (higher index, lower fade) are more transparent.
4. Call `blit` for the main cursor at `top_left` with `alpha_mul = 1.0`, compositing it on top of the trail.

### Behaviors

- `draws_pixels_at_the_position` - a cursor blitted at (20, 20) into a 40x40 zero-filled buffer writes at least one non-zero byte.
- `offscreen_position_is_safe_noop` - blitting at (1000, 1000) into a 40x40 buffer leaves it unchanged (no panic, no out-of-bounds write).

## draw_cursor_posed

```rust
pub fn draw_cursor_posed(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite,
                         pos: (f32, f32), recent: &[(f32, f32)],
                         size_px: f32, blur: f32, bounce: f32, clip: (i32, i32, i32, i32),
                         pose: BusyPose)
```

`draw_cursor` plus pack v2's busy transform (`busy::busy_pose`), applied about the hotspot. `draw_cursor` is now a one-line wrapper passing `BusyPose::still()`.

### Inputs

Everything `draw_cursor` takes, plus:

- `pose: BusyPose` - the frame's busy transform. `BusyPose::still()` for every non-busy cursor, and for a busy one on a v1 pack or a pack shipping explicit frames (those are already the animation).

### Implementation

Identical to `draw_cursor` up to the final blit, which branches:

- `pose.is_identity()` - the existing nearest-neighbour `blit`, **byte-for-byte as before pack v2 existed**. Pinned by `a_still_pose_keeps_the_nearest_neighbour_blit_and_never_interpolates`, which draws at a fractional scale and asserts every painted pixel still holds one of the sprite's ORIGINAL channel values (bilinear sampling would leave in-between greys), and by an equality check against `draw_cursor` itself.
- otherwise - `cursorxform::blit_transformed`, anchored on `pos` rather than the top-left corner.

**The motion trail is never transformed.** It is a fading echo of where the cursor WAS; spinning each ghost independently reads as noise rather than motion.


## apply_enhanced

```rust
pub fn apply_enhanced(
    out: &mut [u8], ow: u32, oh: u32,
    spr: &CursorSprite,
    pos_px: (f32, f32),
    recent: &mut std::collections::VecDeque<(f32, f32)>,
    trail_cap: usize,
    click_ms: &[u32],
    ev_t: u32,
    size: f32,
    blur: f32,
    click_bounce: bool,
    bounce_intensity: f32,
    panel: f32,
    clip: (i32, i32, i32, i32),
    pose: BusyPose,
)
```

Per-frame convenience wrapper: updates the trail deque, computes bounce and pixel size, then calls `draw_cursor`.

### Inputs

- `out`, `ow`, `oh`, `spr`, `clip` - forwarded to `draw_cursor`; see above.
- `pos_px: (f32, f32)` - current hotspot in output pixels. *Why:* pushed to `recent` before drawing so this frame's position becomes the newest trail entry.
- `recent: &mut VecDeque<(f32,f32)>` - rolling trail history owned by `CursorPrep`. *Why mutable:* this function pops the oldest entry when the deque is full and pushes the current position. **It is forward-only state, so its owner has to rewind it:** the deque gains exactly one entry per composited frame, and a preview scrub composites exactly ONE frame per call, so a deque carried across seeks holds the last six SCRUB TARGETS and paints a faded cursor at each - which is why `FrameRenderer::reset_camera` clears it alongside the camera and cursor sims. Pinned by `a_stale_trail_point_draws_a_ghost_until_recent_is_cleared` (unit test): a stale entry paints a ghost at that point; after `recent.clear()` those pixels stay untouched.
- `trail_cap: usize` - maximum trail length in frames. *Why:* bounds the number of blit calls and the memory the deque uses; `cursorset::draw` passes 6.
- `click_ms: &[u32]` - forwarded to `bounce_scale`.
- `ev_t: u32` - current frame event-time; forwarded to `bounce_scale`.
- `size: f32` - user cursor size multiplier, clamped to 0.4..3.0 here. *Why clamped:* rejects unreasonable settings before they propagate into the scale computation.
- `blur: f32` - trail strength; clamped to 0..1 before passing to `draw_cursor`.
- `click_bounce: bool` - enables bounce animation; forwarded to `bounce_scale`.
- `bounce_intensity: f32` - dip depth; forwarded to `bounce_scale`.
- `panel: f32` - ratio of the screen panel width to the full inset width (0.1..1.0). *Why:* when the screen occupies a small PiP corner, the cursor is scaled down proportionally so it stays visually appropriate to the panel size rather than overflowing it.

### Returns

`()`. Modifies `out` and `recent` in place.

### Implementation

1. If `recent.len() >= trail_cap`, pop from the front (drop the oldest position).
2. Push `pos_px` to the back of `recent`.
3. Call `bounce_scale(click_ms, ev_t, click_bounce, bounce_intensity)` to get `bounce`.
4. Compute `size_px = size.clamp(0.4, 3.0) * oh as f32 * 0.033 * panel`. *Why 0.033 * oh:* at size=1.0 and panel=1.0 the cursor occupies ~3.3% of frame height, which is the design baseline calibrated for a 1080p export to look natural.
5. Collect the trail: all `recent` entries except the newest, collected in reverse (newest first).
6. Call `draw_cursor` with the assembled parameters.

- `pose: BusyPose` - forwarded to `draw_cursor_posed`; `cursorset::draw` computes it once per frame from the recorded cursor type and the selected pack.
