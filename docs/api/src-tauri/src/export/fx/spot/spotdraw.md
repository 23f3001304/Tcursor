# src-tauri/src/export/fx/spot/spotdraw.rs

Renders the cursor spotlight effect in all six modes (Classic, Breathing, Vignette, Blur, Nebula, Halo) directly onto a BGRA frame by darkening pixels outside the lit zone and optionally adding a colour tint, then optionally undoes that dim inside the camera panel's rect (the "don't dim the webcam" option). Invoked by `fxdraw::CpuFx::apply` as the second effect layer, after video FX and before click effects.

**`fx.wgsl` is the reference look, not this file.** `select_fx` picks the GPU shader whenever an adapter exists, and since the FX-parity fix the editor preview uses that same selector (`preview_fx.rs`'s `with_fx`), so on any machine with a GPU *neither* the export nor the preview runs this code - it is the fallback for adapter-less machines only. Where the two can diverge, this file must move toward the shader. Two modes stay deliberate approximations because mirroring them per-pixel on the CPU is not affordable: **Blur** (the shader takes 4 extra texture samples per pixel, which needs an unmutated copy of the frame; here it just dims ~15% harder instead) and **Nebula** (the shader evaluates a domain-warped 4-octave fbm per pixel - roughly 100 `sin` calls - which is seconds per frame at 4K; here it is a flat additive tint wash). Everything else - the feather curve, the vignette falloff, the breathing scale, the halo ring including its `intensity` scale, and the camera-exclusion mix - is a line-for-line mirror.

**The rounded rectangle is no longer this file's.** `rrect_cov` used to be a private helper here; it now comes from `export::fx::mask`, which holds the one rounded-rect shape the whole app draws - the camera-exclusion rect, the mask regions and the shader all describe the same corner. Behaviour is unchanged by the move: `dim_camera_false_keeps_camera_rect_lit` and `dim_camera_true_dims_the_camera_rect_too` pass unmodified, which is what pins that the two bodies were identical. See `mask/rrect.md` for the formula, the half-pixel convention and why the bounding-box corner reads as outside the shape once `r > 0`.

## draw_spot

```rust
pub fn draw_spot(out: &mut [u8], ow: u32, oh: u32, s: &Spot, intensity: f32)
```

Modifies `out` in place according to `s.mode`, attenuated by `s.dim * s.alpha`.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame after video FX. *Why mutable:* the spotlight operates as a multiplicative darkening pass directly on the frame; no separate buffer is needed.
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* used to compute the frame centre `(fcx, fcy)`, the vignette normaliser `maxd`, and all size fractions (radii, halo width) that scale with `oh`.
- `s: &Spot` - the full spotlight state snapshot: `s.cx`/`s.cy` are the output-space cursor position; `s.radius_frac` and `s.feather_frac` are fractions of `oh`; `s.dim` is max darkness 0..1; `s.alpha` is the animated fade-in 0..1; `s.mode` selects the algorithm; `s.tint` is the `[r,g,b]` additive colour; `s.t` is the animation phase for Breathing mode; `s.cam_rect`/`s.cam_radius`/`s.dim_camera` describe the camera-panel exclusion (see Implementation step 5). *Why a single struct:* all spotlight parameters are authored together and change together as the user moves; passing one struct keeps the function signature stable as new modes are added.
- `intensity: f32` - `FxState::intensity`, the user's overall FX strength 0..1. *Why it is passed separately rather than living on `Spot`:* it is a whole-`FxState` property shared with the click effects, which is exactly how the shader sees it (`u.c.z`, packed from `state.intensity` by `build_fx_u`, not from the spot). Only Halo reads it here. *Why it matters:* the shader scales the halo ring by `u.c.z` and this path did not, so a user who turned intensity down got a full-strength ring in the CPU/preview path and a faded one in the GPU export.

### Returns

`()`. Returns immediately when `s.dim * s.alpha <= 0.0`. *Why early return:* the per-pixel double loop at 4K is the most expensive path in the CPU FX stack; skipping it entirely when the effect is invisible is important.

### Implementation

1. Compute effective `dim = s.dim.clamp(0,1) * s.alpha.clamp(0,1)`. Return if `<= 0`.
2. If `mode == Breathing`, compute `breathe = 1.0 + 0.12 * sin(s.t * pi)`. Otherwise `breathe = 1.0`. *Why `pi` multiplier:* `s.t` advances at ~3 Hz so one full sine cycle per 1/3 second produces a visible pulse.
3. Compute `r_in = oh * s.radius_frac * breathe` (lit radius in output pixels) and `r_out = r_in + oh * s.feather_frac * breathe` (outer edge of the feather band). `feather_frac` is clamped to minimum 0.001 to prevent `r_in == r_out` and the resulting divide-by-zero.
4. For each pixel `(x, y)`:
   - **Vignette mode**: compute `t` from normalised distance from frame centre: `t = clamp((dist/maxd - 0.4) / 0.6, 0, 1)`. *Why shift-by-0.4:* keeps the inner 40% of the frame diagonal fully lit so the vignette only dims the periphery.
   - **All other modes**: compute `t = clamp((dist - r_in) / (r_out - r_in), 0, 1)` where `dist = hypot(x - s.cx, y - s.cy)`. `t = 0` inside the lit circle, 0..1 in the feather band, 1 outside.
   - Compute `factor`: `0.85` for Nebula (softer dim), `1.15` for Blur (harder dim to compensate for the lack of an actual blur), `1.0` for all others. *These two are the deliberate approximations described at the top of this file; every other branch mirrors `fx.wgsl` exactly.*
   - Compute `k = (1.0 - clamp(dim * factor, 0, 1) * t).max(0.0)`. When `k < 1.0`, multiply each of the 3 BGR channels by `k`. *Why not touch alpha:* the frame's alpha channel is used for downstream compositing; modifying it would break the pipeline.
   - **Nebula extra pass**: for `t > 0`, call `add_tint(out, i, s.tint, 0.25 * t)`. *Why 0.25:* a subtle additive tint wash that shifts colour toward the nebula hue without washing out the content.
   - **Halo extra pass**: compute `band = clamp(1 - |dist - r_in| / (oh * 0.02), 0, 1)` for every pixel. When `band > 0`, call `add_tint` at `band * clamp(intensity, 0, 1)` strength, mirroring the shader's `color + u.tint.rgb * band * u.c.z`. *Why computed for every pixel:* the halo ring sits along `r_in` (the inner lit edge) so pixels both inside and outside the edge contribute to the ring's glow.
5. **Camera-keep pass** (only when `s.dim_camera` is `false`): before any of the above modifies the pixel, snapshot its pre-dim BGR value (`pre`). After the mode-specific dim/tint passes run, compute `cov = rrect_cov(x, y, s.cam_rect.mn, s.cam_rect.mx, s.cam_radius)` and blend `out[i+c] = out[i+c] + (pre[c] - out[i+c]) * cov` per channel - i.e. `mix(dimmed, pre, cov)`, undoing the dim (and any tint just applied) in proportion to how far inside the camera's rounded rect the pixel is. *Why snapshot per-pixel rather than a second full pass:* avoids a second `ow*oh` loop and keeps the exact per-pixel `pre` value regardless of which mode ran, mirroring `fx.wgsl`'s `let pre_spot = color;` snapshot exactly.

### Behaviors

- `vignette_dims_corner_not_center` - a uniform 200-valued frame with `Vignette` mode has a lower byte value at the corner pixel than at the centre pixel.
- `halo_tints_the_ring_edge` - a `Halo` spotlight with a blue-purple tint paints at least one pixel with B > 40 (additive blue at the ring edge).
- `halo_ring_scales_with_intensity_like_the_shader` - the brightest ring pixel dims as `intensity` drops (1.0 → 0.5) and disappears entirely at `intensity = 0`, matching `u.c.z` in `fx.wgsl`. The regression guard for the parity bug this parameter was added to fix.
- `dim_camera_false_keeps_camera_rect_lit` - with `dim_camera = false` and a cam rect far from the spotlight center, the cam rect's *center* pixel (not its bounding-box corner - see `rrect_cov`) stays at full brightness (200) despite being outside the lit spotlight zone.
- `dim_camera_true_dims_the_camera_rect_too` - the same setup with `dim_camera = true` dims that pixel normally, confirming the exclusion is opt-in and today's behavior (default `true`) is unchanged.
