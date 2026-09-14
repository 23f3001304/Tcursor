# src-tauri/src/export/fx/mod.rs

Submodule overviews for the `fx` group.

The group also carries the FX shader itself, which is not a Rust module and so has no doc page of its own: **`fx.wgsl` + `fx_clicks.wgsl`**, two files `fx_gpu::build_pipeline` concatenates (in that order) into ONE wgpu shader module. `fx.wgsl` owns the `FxU` uniform struct, the bindings, the `FX_*`/`SP_*`/`VF_*` id constants, the vertex stage, the noise/nebula helpers, the shockwave UV warp and chromatic dispersion (both of which have to run before the frame is sampled) and the spotlight/video-FX blending; `fx_clicks.wgsl` owns the shared click timing (`fx_ease`, `fx_alpha`), the coverage helpers and `clicks(base, px, oh, style, n)`, the single function `fs_main` calls for every click style. The split is purely the 200-line budget; WGSL resolves module-scope names out of order, so the forward call is legal. See `fx_gpu.md`.

## fx_state

Renderer-agnostic per-frame FX data model, the `FxRenderer` trait, the renderer factory, and the per-frame render entry point. Key items: `FxHit`, `Spot`, `VideoFx`, `FxState`, `FxRenderer` trait, `fx_state_at(fx, events, actions, effects, scene, cam, cur, screen, sw, sh, ow, oh, region_t, ev_t, spot_sim) -> Option<FxState>`, `select_fx(ow, oh) -> Box<dyn FxRenderer>`, `render(...)`.

## spotlight_sim

Private submodule (`mod spotlight_sim;`, no `pub`). The stateful spotlight region resolver, split out of `fx_state.rs` (which was at the size budget) and re-exported as `fx_state::SpotlightSim`. Key items: `region_alpha(region, et) -> f32` (one region's own fade ramp), `SpotlightSim` (tracks the highest-layer active region and eases alpha across a handoff), `SpotlightSim::resolve(effects, et, settings_on) -> f32`, `SpotlightSim::style(effects, fx) -> (SpotlightMode, f32, f32, f32)`.

## fx_uniforms

Packs `FxState` into the `FxU` GPU uniform struct consumed by the FX shader, defining canonical numeric-id mappings for all effect modes plus the hue rotation Neon's second tube is drawn in. Key items: `FxU` (`repr(C)` bytemuck struct, including `color2`), `build_fx_u(state, ow, oh) -> FxU`, `style_id`, `spot_mode_id`, `video_mode_id`, `hue_shift(rgb, deg) -> [f32; 3]`, `MAX_HITS` and `NEON_HUE_SHIFT` constants.

## fx_gpu

GPU-backed `FxRenderer` that uploads the composited frame, runs the FX shader (`fx.wgsl` + `fx_clicks.wgsl`, concatenated into one module: spotlight, click effects, video FX), and reads processed pixels back into the caller's buffer. Key items: `GpuFx` struct, `GpuFx::new(ow, oh) -> Option<GpuFx>`, `GpuFx::apply(out, ow, oh, state)`.

## fxdraw

CPU fallback `FxRenderer` that applies the full effect stack in layer order: video FX then spotlight then click effects. Key items: `CpuFx` (unit struct implementing `FxRenderer`), `CpuFx::apply(out, ow, oh, state)`.

## clickfx

Pure data layer for click effects: computes live hit sets and the two timing curves EVERY click style shares (mirrored by `fx_clicks.wgsl` and by `src/editor/stage/ripplePreview.ts`, all three pinning the same five sample points). Key items: `Hit` (screen-space click with normalized progress), `hits_at(events, et, life_ms)` - returns all live hits at event-time `et`; `ease_out(progress)` (ease-out cubic radii), `fade_alpha(progress, intensity)` (hold then smoothstep release), `ripple_radius(progress, r_max)`, `smoothstep(e0, e1, x)`.

## clickdraw

Renders all click effect styles (Ripple, Pulse, Glow, Neon, Shockwave, Particles) onto a BGRA frame, mirroring `fx_clicks.wgsl` in geometry and gain. Key items: `draw_clicks(out, ow, oh, state)` - dispatches per-hit draws from `FxState`, all sizes as fractions of `oh`, each opening with the shared white impact flash. Shockwave's refraction and chromatic dispersion have no CPU counterpart.

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
