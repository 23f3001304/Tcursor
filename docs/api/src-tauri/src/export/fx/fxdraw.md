# src-tauri/src/export/fx/fxdraw.rs

CPU fallback `FxRenderer` implementation that applies the full effect stack (video FX, spotlight, click effects) sequentially onto the composited BGRA frame. This renderer is always available; the GPU path in `fx_gpu` replaces it when a compatible GPU is detected. `fx_state::select_fx` returns a `Box<dyn FxRenderer>` pointing to either implementation so the rest of the export pipeline never needs to branch.

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

Implements `FxRenderer::apply`. Applies all active effects from `state` onto `out` in place, in a fixed layer order: video FX first, spotlight on top of that, click effects on top of the dimmed result.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame (background + screen + camera already composited). *Why mutable:* every effect modifies pixels in place; no intermediate buffer is allocated.
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* forwarded to each draw function for per-pixel index computation and resolution-scaled sizing.
- `state: &FxState` - the full effect snapshot for this frame: `state.video` drives the video effect, `state.spot` drives the spotlight, `state.hits` / `state.style` / `state.color` / `state.intensity` drive click effects. *Why a single snapshot:* lets the caller freeze state at a single event-time and pass it to any renderer without the renderer querying live data.

### Returns

`()`. Modifies `out` in place.

### Implementation

1. If `state.video` is `Some(v)`, call `videodraw::draw_video(out, ow, oh, &v)`. *Why first:* video effects (colour grade, vignette) are the base layer that affects the entire frame including the area the spotlight will subsequently dim.
2. If `state.spot` is `Some(s)`, call `spotdraw::draw_spot(out, ow, oh, &s)`. *Why second:* the spotlight dims the frame after video colouring, so the lit circle reflects the graded colours rather than the original.
3. Always call `clickdraw::draw_clicks(out, ow, oh, state)`. *Why last:* click rings and particles appear on top of both the video effect and the spotlight dim so they remain bright and visible even in the darkened region.

### Behaviors

- `ripple_paints_a_colored_ring` - a `Ripple` hit at (50, 50) in a 100x100 frame paints at least one pixel with R > 40 (red ring in BGRA at idx 2).
- `spotlight_dims_corner_more_than_center` - with a `Classic` spotlight at centre, the corner pixel is darker than the centre pixel.
- `glow_brightens_near_click` - a `Glow` hit at centre with white colour increases the byte value at the centre pixel above its initial value.
- `neon_paints_a_bright_ring` - a `Neon` hit produces at least one pixel with B > 60 (blue component of the `[0, 128, 255]` colour).
- `shockwave_paints_an_expanding_ring` - a `Shockwave` hit with white colour at `progress = 0.5` paints a visible ring.
- `particles_paint_multiple_specks` - a `Particles` hit at `progress = 0.5` produces more than 3 pixels with B > 40.
- `empty_state_leaves_frame_untouched` - when `hits` is empty and `spot` and `video` are `None`, `out` is unchanged even with a non-None style.
