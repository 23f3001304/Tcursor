# src-tauri/src/export/fx/mask/rrect.rs

The one rounded rectangle in the app. Three effects draw a rounded box - the spotlight's camera-exclusion rect, the mask shape, and (after Batch 2c lands) the animated text plate - and before this file each carried its own copy of the signed-distance formula. One definition means the corner a user sees is the same corner in every one of them, and the WGSL transcription in `fx_mask.wgsl` has exactly one Rust body to be checked against instead of three.

## rrect_sd

```rust
pub fn rrect_sd(x: f32, y: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32
```

The signed distance from the point `(x, y)` to the rounded rectangle bounded by `mn` and `mx` with corner radius `r`, in OUTPUT PIXELS: negative inside, zero on the edge, positive outside, and the magnitude is the real distance in either direction.

*Why a distance rather than a coverage:* the mask feather divides by it (`maskdraw::cov` computes `clamp(0.5 - sd / feather_px, 0, 1)`), so a soft edge of any width falls straight out of the same number. A coverage has already collapsed the distance to a single antialiased pixel and cannot be widened afterwards.

*Why the bounding-box corner reads positive once `r > 0`:* a rounded rect's corner arc is inset by `r` from the box's literal corner, so the pixel at `mn` exactly is legitimately OUTSIDE the shape. That is a property of a real rounded-rect SDF, not an off-by-one (see `a_corner_radius_rounds_the_corner_in_and_nothing_else`, and `spotdraw`'s `dim_camera_false_keeps_camera_rect_lit`, which probes the rect's centre for this reason).

The shape `fx_mask.wgsl`'s `rrect_sd` transcribes line for line; the WGSL takes a `vec2` where this takes two floats and is otherwise identical.

### Used by

- `src-tauri/src/export/fx/mask/maskdraw.rs` (`cov`) - the per-pixel mask coverage, feather included.
- `src-tauri/src/export/fx/mask/rrect.rs` (`rrect_cov`) - the antialiased coverage below.

## rrect_cov

```rust
pub fn rrect_cov(x: f32, y: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32
```

Rounded-rect coverage at pixel `(x, y)`: 1 inside, 0 outside, 0.5 on the edge, with a roughly one-pixel antialiased transition. Exactly `(0.5 - rrect_sd(..)).clamp(0.0, 1.0)`, which is what `spotdraw` computed with its own private copy of the formula before this file existed.

*Why it stays a named function rather than being inlined at the one call site:* it is the half-pixel convention, and a second caller that spelled `0.5 - sd` differently (or clamped after multiplying) would produce a visibly different edge for the same shape.

### Used by

- `src-tauri/src/export/fx/spot/spotdraw.rs` (`draw_spot`) - builds the per-pixel camera-exclusion mask, unchanged in behaviour by the move (`dim_camera_false_keeps_camera_rect_lit` and `dim_camera_true_dims_the_camera_rect_too` pass unmodified, which is the proof).
