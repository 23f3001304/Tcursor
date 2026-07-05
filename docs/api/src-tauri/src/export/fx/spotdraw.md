# src-tauri/src/export/spotdraw.rs

Renders the cursor spotlight effect in all six modes (Classic, Breathing, Vignette, Blur, Nebula, Halo) directly onto a BGRA frame by darkening pixels outside the lit zone and optionally adding a colour tint. Invoked by `fxdraw::CpuFx::apply` as the second effect layer, after video FX and before click effects.

## draw_spot

```rust
pub fn draw_spot(out: &mut [u8], ow: u32, oh: u32, s: &Spot)
```

Modifies `out` in place according to `s.mode`, attenuated by `s.dim * s.alpha`.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame after video FX. *Why mutable:* the spotlight operates as a multiplicative darkening pass directly on the frame; no separate buffer is needed.
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* used to compute the frame centre `(fcx, fcy)`, the vignette normaliser `maxd`, and all size fractions (radii, halo width) that scale with `oh`.
- `s: &Spot` - the full spotlight state snapshot: `s.cx`/`s.cy` are the output-space cursor position; `s.radius_frac` and `s.feather_frac` are fractions of `oh`; `s.dim` is max darkness 0..1; `s.alpha` is the animated fade-in 0..1; `s.mode` selects the algorithm; `s.tint` is the `[r,g,b]` additive colour; `s.t` is the animation phase for Breathing mode. *Why a single struct:* all spotlight parameters are authored together and change together as the user moves; passing one struct keeps the function signature stable as new modes are added.

### Returns

`()`. Returns immediately when `s.dim * s.alpha <= 0.0`. *Why early return:* the per-pixel double loop at 4K is the most expensive path in the CPU FX stack; skipping it entirely when the effect is invisible is important.

### Implementation

1. Compute effective `dim = s.dim.clamp(0,1) * s.alpha.clamp(0,1)`. Return if `<= 0`.
2. If `mode == Breathing`, compute `breathe = 1.0 + 0.12 * sin(s.t * pi)`. Otherwise `breathe = 1.0`. *Why `pi` multiplier:* `s.t` advances at ~3 Hz so one full sine cycle per 1/3 second produces a visible pulse.
3. Compute `r_in = oh * s.radius_frac * breathe` (lit radius in output pixels) and `r_out = r_in + oh * s.feather_frac * breathe` (outer edge of the feather band). `feather_frac` is clamped to minimum 0.001 to prevent `r_in == r_out` and the resulting divide-by-zero.
4. For each pixel `(x, y)`:
   - **Vignette mode**: compute `t` from normalised distance from frame centre: `t = clamp((dist/maxd - 0.4) / 0.6, 0, 1)`. *Why shift-by-0.4:* keeps the inner 40% of the frame diagonal fully lit so the vignette only dims the periphery.
   - **All other modes**: compute `t = clamp((dist - r_in) / (r_out - r_in), 0, 1)` where `dist = hypot(x - s.cx, y - s.cy)`. `t = 0` inside the lit circle, 0..1 in the feather band, 1 outside.
   - Compute `factor`: `0.85` for Nebula (softer dim), `1.15` for Blur (harder dim to compensate for the lack of an actual blur), `1.0` for all others.
   - Compute `k = (1.0 - clamp(dim * factor, 0, 1) * t).max(0.0)`. When `k < 1.0`, multiply each of the 3 BGR channels by `k`. *Why not touch alpha:* the frame's alpha channel is used for downstream compositing; modifying it would break the pipeline.
   - **Nebula extra pass**: for `t > 0`, call `add_tint(out, i, s.tint, 0.25 * t)`. *Why 0.25:* a subtle additive tint wash that shifts colour toward the nebula hue without washing out the content.
   - **Halo extra pass**: compute `band = clamp(1 - |dist - r_in| / (oh * 0.02), 0, 1)` for every pixel. When `band > 0`, call `add_tint` at `band` strength. *Why computed for every pixel:* the halo ring sits along `r_in` (the inner lit edge) so pixels both inside and outside the edge contribute to the ring's glow.

### Behaviors

- `vignette_dims_corner_not_center` - a uniform 200-valued frame with `Vignette` mode has a lower byte value at the corner pixel than at the centre pixel.
- `halo_tints_the_ring_edge` - a `Halo` spotlight with a blue-purple tint paints at least one pixel with B > 40 (additive blue at the ring edge).
