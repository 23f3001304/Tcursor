# src-tauri/src/export/fx/mod.rs

Submodule overviews for the `fx` group.

## fx_state

Renderer-agnostic per-frame FX data model, the `FxRenderer` trait, the renderer factory, and the per-frame render entry point. Key items: `FxHit`, `Spot`, `VideoFx`, `FxState`, `FxRenderer` trait, `fx_state_at(fx, events, actions, scene, cam, cur, sw, sh, ow, oh, et) -> Option<FxState>`, `select_fx(ow, oh) -> Box<dyn FxRenderer>`, `render(...)`.

## fx_uniforms

Packs `FxState` into the `FxU` GPU uniform struct consumed by `fx.wgsl`, defining canonical numeric-id mappings for all effect modes. Key items: `FxU` (`repr(C)` bytemuck struct), `build_fx_u(state, ow, oh) -> FxU`, `style_id`, `spot_mode_id`, `video_mode_id`, `MAX_HITS` constant.

## fx_gpu

GPU-backed `FxRenderer` that uploads the composited frame, runs `fx.wgsl` (spotlight, click effects, video FX), and reads processed pixels back into the caller's buffer. Key items: `GpuFx` struct, `GpuFx::new(ow, oh) -> Option<GpuFx>`, `GpuFx::apply(out, ow, oh, state)`.

## fxdraw

CPU fallback `FxRenderer` that applies the full effect stack in layer order: video FX then spotlight then click effects. Key items: `CpuFx` (unit struct implementing `FxRenderer`), `CpuFx::apply(out, ow, oh, state)`.

## clickfx

Pure data layer for click effects: computes live hit sets and per-hit animation scalars. Key items: `Hit` (screen-space click with normalized progress), `hits_at(events, et, life_ms)` - returns all live hits at event-time `et`; `ripple_radius(progress, r_max)`, `fade_alpha(progress, intensity)`.

## clickdraw

Renders all click effect styles (Ripple, Pulse, Glow, Neon, Shockwave, Particles) onto a BGRA frame. Key items: `draw_clicks(out, ow, oh, state)` - dispatches per-hit draws from `FxState`, all sizes as fractions of `oh`.

## spotdraw

Renders the spotlight effect in six modes (Classic, Breathing, Vignette, Blur, Nebula, Halo) by darkening pixels outside the lit zone. Key items: `draw_spot(out, ow, oh, s)` - modifies `out` in-place, skips the double loop entirely when `s.dim * s.alpha <= 0`.

## spotlight

Thin typed adapter over `hold::hold_alpha` for the spotlight effect. Key items: `hold_alpha(actions, et, fade_ms)` - returns spotlight strength 0..1 ramping around `SpotlightHoldStart`/`SpotlightHoldEnd` action pairs.

## caption

Renders keyboard-shortcut chord captions using embedded Inter SemiBold, centered in the bottom band of the frame with a drop shadow. Key items: `overlay(out, ow, oh, actions, keys, et, enabled)` - per-frame entry point; `caption_at(actions, keys, et, life_ms)` - returns active caption text and fade alpha; `draw_caption(out, ow, oh, text, alpha)`.

## videodraw

Applies full-frame video effects (CinematicDim, ScreenFocus, ColorPop, NebulaWash) in software onto the composited BGRA frame. Key items: `draw_video(out, ow, oh, v)` - CPU fallback for all `VideoFxMode` variants, all effects expressed as per-pixel multiplications or additions scaled by `v.alpha`.

## hold

Generic fade-in/hold/fade-out alpha ramp shared by spotlight and video FX, accepting predicate closures for start and end action kinds. Key items: `hold_alpha(actions, et, fade_ms, is_start, is_end) -> f32`.
