# src-tauri/src/export/fx/caption/mod.rs

Submodule overviews for the SPOKEN caption group (the transcript track, not the hotkey chord overlay - that is `click::hotkeycap`). `captionlayout` decides where a caption sits, `captiondraw` puts it there. The split is the parity line: the layout numbers are shared with the TypeScript preview, the blit is not.

## captionlayout

Where a SPOKEN caption sits AND how far into its entry or exit it is: a pure function shared, number for number, with the preview's `src/editor/stage/fx/captionPreview.ts` (ADDED-5 - parity over layout numbers, not pixels). Key items: the constants `ADV`, `LINE_H`, `PAD_X`, `PAD_Y`, `MARGIN`, `RISE_LINES`, `POP_FROM`, `MAX_CHARS`, `MAX_LINES`; `LaidCaption`; `ease_out(p)` and `ramp(dt, ms)`; `wrap_lines(text, max)`; `caption_at(caps, t_ms) -> Option<&Caption>`; `word_span(cap, i) -> Option<(usize, usize)>`; `layout(cap, style, ow, oh, t_ms) -> LaidCaption`. The fade length is no longer a constant here: it is `CaptionStyle::animation_ms`, defaulting to the old 120. See `caption/captionlayout.md`.

## captiondraw

The spoken-caption blit: the scrim pill in `style.pill_color` at `style.pill_alpha`, the centred glyph run in `style.text_color`, and `style.highlight_color.unwrap_or(accent)` on the word being spoken - all positioned, coloured by alpha and revealed word by word according to the `LaidCaption` `captionlayout` returned. Reuses `click::hotkeycap`'s embedded Inter SemiBold and its `put` blend. Called from `FrameRenderer::composite_at` directly rather than through `fx_state::render` (that function is already at eighteen arguments), which is also why GIF keeps captions for free. Key items: `overlay(out, ow, oh, caps, style, accent, t_ms)`. See `caption/captiondraw.md`.
