# src-tauri/src/export/render/composite.rs

The per-frame half of `FrameRenderer`: one `impl` block holding the composite pass and the lens build it needs. Split out of `render/mod.rs`, which now holds the struct and the one-time `new`, and it follows the same convention as the sibling `accessors.rs` and `step.rs` - the type is declared once and its methods are grouped by the job they do.

## FrameRenderer::composite_at

```rust
pub fn composite_at(&mut self, pose: &FramePose, screen: &[u8], prev: Option<&[u8]>,
                    webcam: Option<(&[u8], u32, u32)>, out: &mut Vec<u8>)
```

`prev` is the screen frame the caller latched before the mid-take display switch this frame is inside (`FramePose::hold` says which frame to latch, `FramePose::mix` says a switch is in flight), and `None` on every frame outside one - which is every frame of a take that never switched. When both are present the held frame is resampled into the current span's rect and blended under it (`render::screen_mix`) BEFORE the compositor runs, so the cross-dissolve needs no second screen input in either compositor and the exporter and the one-shot preview reach it through one path.

## FrameRenderer::lenses

```rust
fn lenses(&self, pose: &FramePose, ow: u32, oh: u32, sc: &crate::export::scene::Panel, inset_w: f32) -> Option<crate::export::fx::fx_lens::Lenses>
```

The glass lenses for this frame, or `None` when the cursor pack does not want one (`fx_lensbuild::wants_lens`, which also answers `None` for a plain OS cursor). Private, and called from `composite_at` only when the captured-cursor path is NOT drawing - a captured cursor is already the real pixels, so magnifying them twice would be wrong.
