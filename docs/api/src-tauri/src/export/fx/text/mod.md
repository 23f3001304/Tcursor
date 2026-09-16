# src-tauri/src/export/fx/text/mod.rs

The animated text overlay: the four looks, the shared pure layout the preview mirrors, and the CPU blit that paints them.

It is a folder rather than two loose files at the top of `export/fx/` for the reason `click`, `spot`, `lens` and `caption` are folders: that directory already holds more than four files, and an overlay's table, layout and blit belong together.

Text is the one Batch 2 feature that is NOT per-pixel work, so it does not ride the FX pass and has no shader. It is a CPU `ab_glyph` blit through `export/fx/glyph.rs`, called from `FrameRenderer::fx_pass` AFTER `fx_state::render` and BEFORE `captiondraw::overlay`. That position is the ruling: after the effects, so a grade or a spotlight does not tint a title whose colour the user chose; before the captions, so nothing ever covers a caption (spec 4 fit point 1). The cursor is blitted after the whole seam returns and therefore stays on top of both.

## text_style

The four looks as a fixed table: `clean`, `plate`, `accent` and `bar`. Key items: `Fill`, `TextStyle`, `style_of(name)` (anything unknown degrades to `clean`, mirroring the ops layer's own coercion), `fill_rgb(fill, accent)` (the accent is read from `Settings.ui.accent` at draw time and never stored on the item). See `text_style.md`.

## textlayout

The shared pure function: where the block sits, how big it is and how far into its animation it is, with nothing in it that touches a frame or a font file. Key items: `LaidText`, `lay_one(item, accent, ow, oh, t_ms)`, `texts_at(..)`, the re-exported `ADV` and `LINE_H` (captions' constants, owned by whichever feature landed first) and the seven look constants `SUB_RATIO`, `PAD_X`, `PAD_Y`, `MARGIN`, `RULE_W`, `SLIDE_FRAC`, `POP_FROM`. `src/editor/stage/text/textPreview.ts` implements the identical contract and both sides pin the same generated 48-row table. See `textlayout.md`.

## textdraw

The CPU blit: the plate, the rule, the main run and the sub run, onto the composited BGRA frame, through the shared `glyph` module. Key item: `overlay(out, ow, oh, items, accent, t_ms)`, called from `FrameRenderer::fx_pass` after the effects and before the captions. It also carries a private `rrect_sd` that the controller replaces with Track 2a's shared one at integration (Shared file ledger, last row). See `textdraw.md`.
