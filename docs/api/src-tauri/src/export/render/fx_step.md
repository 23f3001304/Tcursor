# src-tauri/src/export/render/fx_step.rs

The FX step of the frame, moved out of `composite_at` verbatim into one `impl` block of its own. Nothing about the picture changed when it moved: the same two calls, the same arguments, the same order.

It exists as its own file because of who edits it next. The parity features (masks, colour grade, animated text) are built by an agent each, in parallel, and every one of them ends in the same place - one more call in the middle of the frame. Left inside `composite_at` that is three agents editing the same hundred-line function and three merges to reconcile; here it is three one-line additions to a file that holds nothing else, and the ordering decision stays in one pair of hands instead of being an accident of who merged last.

The order those calls land in, ruled by the controller (spec 1.3, `docs/superpowers/specs/2026-09-15-editor-parity-features-design.md`), is inside `fx_pass` and in full:

1. **Batch 2a's masks** - blur, pixelate, highlight. First, because a mask hides something in the PICTURE, and everything after it should treat the hidden pixels as if they had always looked that way.
2. **Batch 2b's grade** - exposure, contrast, vignette. After the masks, so a blurred password takes the same grade as the pixels around it rather than a differently-lit patch; before `fx_state::render`, so the grade covers the picture (background, screen, webcam) and not the overlays whose colour the user picked - a tinted spotlight or a click ring the grade had darkened would no longer be the colour the settings show.
3. **`fx_state::render`** - spotlight dim, video FX, click effects, and the cursor lens.
4. **Batch 2c's animated text** - titles, lower thirds, stats, callouts. After the effects: text is authored content the viewer reads, not part of the picture being graded.
5. **`captiondraw::overlay`** - the spoken captions, last in this file.

`composite_at` draws the cursor AFTER this seam returns, so the cursor is topmost and BOTH the animated text and the captions sit under it - the same rationale `render/mod.md` already gives for the captions: a caption belongs over the picture and its effects but never over the pointer.

## FrameRenderer::fx_pass

```rust
pub(super) fn fx_pass(&mut self, pose: &FramePose, out: &mut [u8], ow: u32, oh: u32,
                      lens: Option<crate::export::fx::fx_lens::Lenses>)
```

Draws everything between the compositor and the cursor, over the composited frame in `out`: `fx_state::render` (spotlight dim, video FX, click effects, and the cursor lens when there is one, on the GPU through `fx.wgsl` or on the CPU fallback), then `captiondraw::overlay`.

`lens` is passed in rather than built here because the caller already knows whether it wants one: a captured OS cursor is the real pixels, so `composite_at` builds no lens for it and hands `None` down (see `FrameRenderer::lenses`).

`pub(super)` - `composite_at` is the only caller, and the frame's passes are `render`'s business, not the exporter's.
