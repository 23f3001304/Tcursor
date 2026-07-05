# src-tauri/src/export/videodraw.rs

Applies full-frame video effects (cinematic dim, screen focus, colour pop, nebula wash) directly onto the composited BGRA frame in software. This is the CPU path - the same visual results are achievable via the GPU shader when available; this file is the guaranteed fallback for every export. Invoked by `fxdraw::CpuFx` before spotlight and click effects so video FX form the base enhancement layer.

## draw_video

```rust
pub fn draw_video(out: &mut [u8], ow: u32, oh: u32, v: &VideoFx)
```

Modifies every relevant pixel of `out` in-place according to `v.mode`, scaled by `v.alpha`.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame (background + screen + camera already composited). *Why mutable in-place:* avoids a second frame-sized allocation at 4K; each mode is multiplicative or additive over existing pixels.
- `ow: u32` - output frame width in pixels. *Why:* used for per-pixel index computation and for deriving the `ScreenFocus` margin width as 8% of `ow`.
- `oh: u32` - output frame height in pixels. *Why:* used for the vignette radius normaliser `maxd` and the `ScreenFocus` margin height.
- `v: &VideoFx` - the animated effect state: `mode` selects the algorithm; `alpha` (0..1) scales the effect strength so it can be faded in and out over time. *Why bundle mode + alpha:* keeps the function signature stable as new modes are added to `VideoFxMode`.

### Returns

`()`. Exits immediately without touching `out` when `v.alpha <= 0.0`. *Why early return:* the per-pixel loop costs 40-80 ms at 4K; skipping it entirely at zero alpha is important for performance.

### Implementation

1. Clamp `v.alpha` to 0..1; return early if zero.
2. Compute frame centre `(fcx, fcy)` and the vignette normaliser `maxd = sqrt(fcx^2 + fcy^2)` (diagonal from centre to corner), clamped to minimum 1.0 to prevent division by zero on degenerate frames.
3. Compute `ScreenFocus` margin widths `(mx, my)` as 8% of each dimension.
4. Iterate every pixel `(x, y)` and compute its BGRA byte offset `i`:
   - `CinematicDim` - computes `vg = hypot(x - fcx, y - fcy) / maxd` (0 at centre, ~1 at corner); multiplies each RGB channel by `k = 1 - alpha * (0.2 + 0.5 * vg)`. *Why 0.2 + 0.5 * vg:* centre pixels are dimmed by 20% at full alpha (preserves visibility) while corners reach 70% dimming, mimicking a cinema-style vignette without letterboxing.
   - `ScreenFocus` - a pixel is "inside" if it lies within the 8% margin on all four sides. Outside pixels are dimmed by `k = 1 - alpha * 0.6`. *Why 8% margin:* keeps the effect sharp enough to feel like a directed spotlight while leaving most of the screen untouched.
   - `ColorPop` - computes Rec.601 luminance `l = 0.299*R + 0.587*G + 0.114*B`; for each channel blends `(channel - l) * 1.6 + l` against the original at `alpha`. *Why Rec.601:* perceptually weighted luminance preserves apparent brightness while the 1.6x stretch around it boosts saturation without hue shift.
   - `NebulaWash` - blends a hard-coded tint `[120, 90, 255]` (BGR) at `alpha * 0.4`. *Why a flat tint instead of fbm:* a per-pixel fractional brownian motion at 4K is impractical on CPU; the GPU path handles the real animated nebula; this ensures the hue family is preserved even on CPU-only exports.

### Behaviors

- `cinematic_darkens_corner` - a uniform 200-valued frame has a measurably lower byte value at the top-left corner (high vg) after `CinematicDim` at alpha 1.0.
- `focus_dims_edge_not_center` - the top-left pixel (outside the margin) is darker than the centre pixel (inside) after `ScreenFocus` at alpha 1.0.

### Used by

- `src-tauri/src/export/fxdraw.rs` - `CpuFx::apply` calls `draw_video` as the first of three sequential effect layers when `state.video` is `Some`.
