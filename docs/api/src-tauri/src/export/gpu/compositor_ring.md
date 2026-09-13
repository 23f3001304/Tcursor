# src-tauri/src/export/gpu/compositor_ring.rs

The webcam panel's ring/border band, split out of `compositor.rs` so that file stays under its 200-line budget. Pure pixel work moved verbatim: no behavior change.

## blit_ring

```rust
pub(super) fn blit_ring(dst: &mut [u8], dw: u32, dh: u32, pw: u32, ph: u32, ox: i32, oy: i32,
                        r: f32, ring_px: f32, ring_color: [u8; 3], a: f32)
```

Blends `ring_color` over `dst` in a band just inside the panel edge, width `ring_px`, weighted by the panel's alpha `a`. The band is `d in [-ring_px, 0]` of the same rounded-box SDF `compositor::rrect_sd_px` computes, which is the same formula and the same band the WGSL shader uses after its camera mix - that is what keeps the CPU and GPU paths drawing the same ring.

`ox`/`oy` are SIGNED: a panel-local pixel whose destination lands off any edge (negative as well as `>= dw`/`>= dh`) is skipped, so an off-canvas panel clips to its visible sub-rect instead of translating onto the destination's corner.

Destination bytes are BGRA; `ring_color` is RGB, hence the channel order in the three blends.

### Used by

- `src-tauri/src/export/gpu/compositor.rs` (`draw_panel`) - the only caller, when `panel.ring_px > 0.0`.
- `src-tauri/src/export/gpu/shader.wgsl` - the GPU twin of this blend; the two must stay in step.
