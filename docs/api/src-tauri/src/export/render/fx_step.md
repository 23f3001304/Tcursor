# src-tauri/src/export/render/fx_step.rs

The FX step of the frame, moved out of `composite_at` verbatim into one `impl` block of its own. Nothing about the picture changed when it moved: the same calls, the same arguments, the same order.

It exists as its own file because of who edits it next. The parity features (masks, colour grade, animated text) were built by an agent each, in parallel, and every one of them ended in the same place - one more thing in the middle of the frame. Left inside `composite_at` that would have been three agents editing the same hundred-line function and three merges to reconcile; here it is a file that holds nothing else, and the ordering decision stayed in one pair of hands instead of being an accident of who merged last.

The order, ruled by the controller (spec 1.3, `docs/superpowers/specs/2026-09-15-editor-parity-features-design.md`) and now in the code, is inside `fx_pass` and in full:

1. **`fx_state::render`**, which RECEIVES the masks and the grade and draws them ahead of everything else in the pass: the **masks** (blur, pixelate, highlight) first, because a mask hides something in the PICTURE and everything after it should treat the hidden pixels as if they had always looked that way; then the **grade** (exposure, contrast, vignette), so a blurred password takes the same grade as the pixels around it rather than showing as a differently-lit patch. Both arrive as arguments (`masks,` then `self.grade,`) rather than as calls of their own, because each is per-pixel work on every output pixel and this pass is the one place the frame is already bound as a texture; a call of its own would be a second full read and write of the frame for nothing, and for blur and pixelate it is the only way the GPU can re-sample the composited frame at all. `fs_main` in `fx.wgsl` runs `mask_fx` then `grade_fx`; `CpuFx::apply` runs `draw_masks` then `draw_grade`; the two agree step for step.
2. **The rest of `fx_state::render`** - the spotlight dim, the video FX, the click effects and the cursor lens - over the masked and graded picture. All of it is AFTER the grade, so the grade covers the picture (background, screen, webcam) and not the things whose colour the user picked: a tinted spotlight or a click ring the grade had darkened would no longer be the colour the settings show.
3. **`textdraw::overlay`** - the animated text: titles, lower thirds, stats, callouts. After the effects, because text is authored content the viewer reads, not part of the picture being graded.
4. **`captiondraw::overlay`** - the spoken captions, last in this file.

`composite_at` draws the cursor AFTER this seam returns, so the cursor is ALWAYS topmost and BOTH the animated text and the captions sit under it - the same rationale `render/mod.md` already gives for the captions: a caption belongs over the picture and its effects but never over the pointer.

## FrameRenderer::fx_pass

```rust
pub(super) fn fx_pass(&mut self, pose: &FramePose, out: &mut [u8], ow: u32, oh: u32,
                      lens: Option<crate::export::fx::fx_lens::Lenses>)
```

Draws everything between the compositor and the cursor, over the composited frame in `out`, in this order:

1. The mask list for this frame, built here and handed straight on.
2. `fx_state::render` - the masks and then the colour grade first of all, then spotlight dim, video FX, click effects, and the cursor lens when there is one, on the GPU through `fx.wgsl` or on the CPU fallback.
3. `textdraw::overlay` - the animated text items (`self.texts`, `self.settings.ui.accent`, `pose.out_t`). **After** the effects, so a grade, a spotlight scrim or a video effect does not tint a title whose colour the user chose; text is authored content the viewer reads, not part of the picture being graded.
4. `captiondraw::overlay` - the spoken captions, and always last.

**The masks are built here**, by `fx_masks::masks_at(&self.effects, &pose.scene, pose.cam, coordmap::full_src(self.sw, self.sh), ow, oh, pose.out_t, self.settings.clickfx.spotlight_dim)`, and handed to `fx_state::render` as its `masks` argument, just ahead of `self.grade` - the same shape `lens` already has, and for the same reason: none of the three is a click effect, and anything built inside `fx_state_at` vanishes when a user turns click animations off.

Two arguments are worth naming:

- `coordmap::full_src(self.sw, self.sh)` is the WHOLE RECORDED CANVAS, which is what a mask's stored `[x, y, w, h]` fractions are fractions of. It is deliberately not `pose.scene.src`, the slice of that canvas currently being shown: using the live slice would make a mask drift whenever a mid-take display switch changed it, where using the full canvas makes the mask stay put and correctly fall off the panel instead.
- `self.effects` is already on the OUTPUT clock, because `EditState::load` runs `remap_doc` before the renderer sees the document, so `pose.out_t` is the right time to sample the fades at.

`self.grade` is passed as an argument of `fx_state::render` rather than being read from `self.settings.grade` here, because it is already resolved to `GradeParams` (`EditState::load`) and because `None` is the signal that there is no look at all - the FX pass is then not built for the grade's sake and an ungraded project costs exactly what it did before the grade existed. It is inside `fx_state::render`, not beside it, for the same reason the spotlight is: the grade is per-pixel work on the frame the pass already has bound as a texture.

`lens` is passed in rather than built here because the caller already knows whether it wants one: a captured OS cursor is the real pixels, so `composite_at` builds no lens for it and hands `None` down (see `FrameRenderer::lenses`).

The text step sits BEFORE the captions because of spec 4's fit point 1: captions are drawn last and nothing may cover them. So a text item the user parks at the bottom centre COLLIDES with a caption rather than winning, and nothing here moves anything to avoid it - the Text inspector hints the collision instead, because silently relocating something the user positioned is worse than letting them see the overlap.

`composite_at` draws the cursor after this function returns, so the cursor is still topmost and both the text and the captions sit under it.

The order is pinned by `export/fx/text/textdraw_tests.rs::the_pass_order_is_text_over_the_effects_and_under_the_captions`, which `include_str!`s this file and asserts the three call sites appear in that sequence. A source-order assertion is a blunt instrument and it is there on purpose: the alternative is an end-to-end frame render inside a unit test, and this fails loudly the moment someone reorders the seam.

`pub(super)` - `composite_at` is the only caller, and the frame's passes are `render`'s business, not the exporter's.
