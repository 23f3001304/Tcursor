# src-tauri/src/export/fx/click/mod.rs

Submodule overviews for the click group: everything an input event puts on the frame. `clickfx` is the data layer both renderers and the TypeScript preview agree on, `clickdraw` is the CPU renderer, and `hotkeycap` is the keyboard-chord overlay - the other input-driven thing drawn over the picture, and the owner of the embedded Inter SemiBold that `caption::captiondraw` blits with.

## clickfx

Pure data layer for click effects: computes live hit sets and the two timing curves EVERY click style shares (mirrored by `fx_clicks.wgsl` and by `src/editor/stage/ripplePreview.ts`, all three pinning the same five sample points). Key items: `Hit` (screen-space click with normalized progress), `hits_at(events, et, life_ms)` - returns all live hits at event-time `et`; `ease_out(progress)` (ease-out cubic radii), `fade_alpha(progress, intensity)` (hold then smoothstep release), `ripple_radius(progress, r_max)`, `smoothstep(e0, e1, x)`. See `click/clickfx.md`.

## clickdraw

Renders all click effect styles (Ripple, Pulse, Glow, Neon, Shockwave, Particles) onto a BGRA frame, mirroring `fx_clicks.wgsl` in geometry and gain. Key items: `draw_clicks(out, ow, oh, state)` - dispatches per-hit draws from `FxState`, all sizes as fractions of `oh`, each opening with the shared white impact flash. Shockwave's refraction and chromatic dispersion have no CPU counterpart. See `click/clickdraw.md`.

## hotkeycap

Renders keyboard-shortcut chord captions (the HOTKEY overlay, not the spoken-caption track) using embedded Inter SemiBold, centered in the bottom band of the frame with a drop shadow. Renamed from `caption.rs` (ADDED-3, M5 T1). Key items: `overlay(out, ow, oh, actions, keys, et, enabled)` - per-frame entry point; `caption_at(actions, keys, et, life_ms)` - returns active caption text and fade alpha; `draw_caption(out, ow, oh, text, alpha)`. Its `FONT` bytes and its `put` blend are `pub(crate)` so `caption::captiondraw` shares them rather than embedding a second copy of Inter and writing a second blend. See `click/hotkeycap.md`.
