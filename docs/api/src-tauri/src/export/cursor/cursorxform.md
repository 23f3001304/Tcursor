# src-tauri/src/export/cursor/cursorxform.rs

Rotated/scaled cursor blit, for pack v2's animated busy state (`busy.rs`). `cursordraw`'s own `blit` is a nearest-neighbour axis-aligned copy and stays the fast path for every still cursor; this file is only reached when `busy_pose` asks for a real transform, so its extra cost is confined to the one animated sprite in the frame.

**Destination-driven.** Each output pixel is inverse-mapped back into sprite space and bilinearly sampled. Walking the source forward instead would leave seams at any angle that is not a multiple of 90 degrees.

## blit_transformed

```rust
pub fn blit_transformed(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite, anchor: (f32, f32),
                        scale: f32, angle_deg: f32, extra: f32, clip: (i32, i32, i32, i32))
```

Alpha-blit `spr` rotated `angle_deg` CLOCKWISE and scaled by `extra` about its hotspot, with the hotspot itself landing on `anchor`.

### Inputs

- `out`, `ow`, `oh` - the composited frame, BGRA.
- `anchor: (f32, f32)` - where the hotspot goes, in output px. **The same anchor the untransformed blit uses**, which is what stops an animating busy cursor drifting off the cursor point as it spins.
- `scale: f32` - the sprite's own output scale (sprite px to output px), exactly as `draw_cursor_posed` computes it from `size_px / canvas_h`.
- `angle_deg`, `extra` - the animation's rotation and its scale ON TOP of `scale`, from `BusyPose`.
- `clip` - the screen-panel box, applied exactly as in `blit`.

### Implementation

1. `bounds` maps the sprite's four corners forward and clamps to the frame and the clip box, so the per-pixel loop never walks the rest of the frame.
2. Per output pixel: subtract `anchor`, rotate by `-angle` (the inverse of the forward clockwise map, in y-down screen coordinates), divide by `scale * extra`, add the hotspot back - giving a point in sprite pixels.
3. `sample` bilinearly, `blend` source-over.

A degenerate call (`scale <= 0`, `extra <= 0`, a sub-pixel sprite, an anchor far off-frame) returns without writing.

## bounds

```rust
fn bounds(sw: f32, sh: f32, hot: (f32, f32), anchor: (f32, f32), total: f32, sin: f32, cos: f32,
          ow: u32, oh: u32, clip: (i32, i32, i32, i32)) -> (i32, i32, i32, i32)
```

The output rectangle the transformed sprite can touch: its four corners mapped FORWARD (the inverse of the per-pixel map), then clamped to the frame and the clip box. Returned as `(x0, y0, x1, y1)`; an empty or inverted result makes the caller a no-op, the same convention `blit` uses.

## sample

```rust
fn sample(spr: &CursorSprite, x: f32, y: f32) -> Option<[f32; 4]>
```

Bilinear sample at sprite-space `(x, y)` (already shifted to texel centres), returned as **premultiplied** BGRA floats. `None` outside the sprite, or where the result is effectively transparent.

*Why premultiplied:* interpolating straight-alpha colour pulls the RGB of fully transparent texels into the edge, haloing every rotated sprite with whatever its padding happens to be (black, for a cursor PNG). Weighting colour by alpha before interpolating is what removes that. Pinned by `cursorxform_tests::a_rotated_edge_blends_without_a_halo_from_transparent_padding`, which composites a white texel over a black frame and asserts each channel stays equal to the coverage it was blended at.

## blend

```rust
fn blend(out: &mut [u8], ow: u32, ox: i32, oy: i32, px: [f32; 4])
```

Source-over blend of one premultiplied BGRA sample onto the frame.

**Rounds rather than truncating**, which `blit`'s integer-aligned fast path can afford to do: bilinear weights never land exactly on 1 (`sin(360deg)` is not quite 0 in `f32`), so an untruncated full-coverage pixel would come out one step dark on every channel - visible as a dimmed rim right around a rotating cursor, and the reason `a_full_turn_is_the_identity_and_a_half_turn_reverses_both_axes` can assert exact equality.

### Used by

- `src-tauri/src/export/cursor/cursordraw.rs` - `draw_cursor_posed`, only when `BusyPose::is_identity` is false

### Preview parity

The canvas mirror is `drawPosed` in `src/editor/stage/cursorPreview.ts`: a `translate`/`rotate`/`scale` about the cursor point, which is the same transform about the same anchor. The preview relies on Canvas2D's own resampling rather than reimplementing the bilinear loop, so the two are geometrically identical but not pixel-identical - which is the existing arrangement for every other part of the preview.
