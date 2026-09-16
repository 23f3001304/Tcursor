# src-tauri/src/export/fx/fxdraw.rs

CPU fallback `FxRenderer` implementation that applies the full effect stack (masks, the colour grade, video FX, spotlight, click effects) sequentially onto the composited BGRA frame. This renderer is always available; the GPU path in `fx_gpu` replaces it when a compatible GPU is detected. `fx_state::select_fx` returns a `Box<dyn FxRenderer>` pointing to either implementation so the rest of the export pipeline never needs to branch.

## CpuFx

```rust
pub struct CpuFx;
```

Unit struct implementing the `FxRenderer` trait. Carries no state; all rendering inputs arrive via `FxState` on each call.

### Used by

- `src-tauri/src/export/fx/fx_state.rs` - `select_fx` returns `Box::new(CpuFx)` when no GPU is available.

## CpuFx::apply

```rust
fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState)
```

Implements `FxRenderer::apply`. Applies all active effects from `state` onto `out` in place, in a fixed layer order: masks first, then the colour grade, then video FX, spotlight on top of that, click effects on top of the dimmed result. The order mirrors `fs_main` in `fx.wgsl` step for step, which is what keeps the CPU fallback and the GPU path producing the same frame.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame (background + screen + camera already composited). *Why mutable:* every effect modifies pixels in place; no intermediate buffer is allocated.
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* forwarded to each draw function for per-pixel index computation and resolution-scaled sizing.
- `state: &FxState` - the full effect snapshot for this frame: `state.masks` drives the mask blit, `state.grade` drives the colour grade, `state.video` drives the video effect, `state.spot` drives the spotlight, `state.hits` / `state.style` / `state.color` / `state.intensity` drive click effects. *Why a single snapshot:* lets the caller freeze state at a single event-time and pass it to any renderer without the renderer querying live data.

### Returns

`()`. Modifies `out` in place.

### Implementation

1. Always call `maskdraw::draw_masks(out, ow, oh, &state.masks)`. *Why first:* a mask hides something in the PICTURE, so everything after it should treat the hidden pixels as if they had always looked that way - a blurred password takes the same grade and the same spotlight dim as the pixels around it, rather than showing as a differently-lit patch. This mirrors `fx.wgsl`, where `mask_fx` runs on the sampled colour before the grade, the video-FX and the spotlight blocks. The call is unconditional because `draw_masks` returns on an empty list.
2. If `state.grade` is `Some(g)`, call `grade::gradedraw::draw_grade(out, ow, oh, &g)`. *Why second:* the grade is part of the PICTURE (spec 1.3), so it must land before anything the user authored a colour for. A Noir grade that desaturated a tinted spotlight or a click ring would read as a bug, and the settings panel would stop showing the colour the frame actually has.
3. If `state.video` is `Some(v)`, call `videodraw::draw_video(out, ow, oh, &v)`. *Why here:* the full-frame video washes are the base EFFECT layer, over the graded picture and under the area the spotlight will subsequently dim. They are a separate thing from `state.grade`: a wash is a timed effect region the user placed, the grade is the project's standing look.
4. If `state.spot` is `Some(s)`, call `spotdraw::draw_spot(out, ow, oh, &s, state.intensity)`. *Why here:* the spotlight dims the frame after the grade and the video colouring, so the lit circle reflects the graded colours rather than the original. *Why `intensity` is forwarded separately:* it lives on `FxState`, not on `Spot` (the shader reads it the same way, as `u.c.z`), and Halo mode scales its ring by it.
5. Always call `clickdraw::draw_clicks(out, ow, oh, state)`. *Why last:* click rings and particles appear on top of the grade, the video effect and the spotlight dim so they remain bright and visible even in the darkened region.

### Behaviors

- `ripple_paints_a_colored_ring` - a `Ripple` hit at (50, 50) in a 100x100 frame paints at least one pixel with R > 40 (red ring in BGRA at idx 2).
- `pulse_paints_a_disc_with_a_white_core` - a `Pulse` hit at `progress = 0.2` puts a white core at the click point (both the tint's own channel and the channels the tint has none of are lit) while a pixel out on the disc's rim is the tint alone. Pins that the core and the rim are separate passes.
- `every_style_flashes_white_at_the_instant_of_the_click` - at `progress = 0` with a BLACK tint - which contributes nothing on a purely additive style - Glow, Shockwave, Particles and Neon all still light the click point, because the impact flash is style-independent. *Why Ripple and Pulse are not probed this way:* they composite their tint OVER the flash (`blend`, not `add_blend`), so a black one would paint it back out at the exact centre; their own tests cover them, and `fx_gpu_tests.rs` pins the shared curve on the shader.
- `spotlight_dims_corner_more_than_center` - with a `Classic` spotlight at centre, the corner pixel is darker than the centre pixel.
- `glow_brightens_near_click` - a `Glow` hit at centre with white colour increases the byte value at the centre pixel above its initial value.
- `neon_paints_a_bright_ring` - a `Neon` hit produces at least one pixel with B > 60 (blue component of the `[0, 128, 255]` colour).
- `shockwave_paints_an_expanding_ring` - a `Shockwave` hit with white colour at `progress = 0.5` paints a visible ring.
- `particles_paint_multiple_specks` - a `Particles` hit at `progress = 0.5` produces more than 3 pixels with B > 40.
- `empty_state_leaves_frame_untouched` - when `hits` and `masks` are empty and `spot` and `video` are `None`, `out` is unchanged even with a non-None style.
