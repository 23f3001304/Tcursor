# src-tauri/src/export/fx/mod.rs

Submodule overviews for the `fx` group. Four of them are folders - `click`, `spot`, `lens` and `caption` - each holding one overlay's data model and its renderers; the files at this level are the ones every overlay shares: the per-frame state, the GPU uniform packing, and the two `FxRenderer` implementations.

Three module aliases live here so callers outside `fx` keep the paths they had before the folders existed: `pub use self::lens as fx_lens`, `pub use self::lens::build as fx_lensbuild` and `pub use self::caption::captiondraw`. INVARIANT: they exist only for `export/cursor` and `export/render`; nothing inside `fx` may use them.

The group also carries the FX shader itself, which is not a Rust module and so has no doc page of its own: **`fx.wgsl` + `fx_clicks.wgsl`**, two files `fx_gpu::build_pipeline` concatenates (in that order) into ONE wgpu shader module. `fx.wgsl` owns the `FxU` uniform struct, the bindings, the `FX_*`/`SP_*`/`VF_*` id constants, the vertex stage, the noise/nebula helpers, the shockwave UV warp and chromatic dispersion (both of which have to run before the frame is sampled) and the spotlight/video-FX blending; `fx_clicks.wgsl` owns the shared click timing (`fx_ease`, `fx_alpha`), the coverage helpers and `clicks(base, px, oh, style, n)`, the single function `fs_main` calls for every click style. The click styles are the half that changes, so they are the half kept on its own; WGSL resolves module-scope names out of order, so the forward call is legal. See `fx_gpu.md`.

## fx_state

Renderer-agnostic per-frame FX data model, the `FxRenderer` trait, the renderer factory, and the per-frame render entry point. Key items: `FxHit`, `Spot`, `VideoFx`, `FxState`, `FxRenderer` trait, `fx_state_at(fx, events, actions, effects, scene, cam, cur, screen, sw, sh, ow, oh, region_t, ev_t, spot_sim) -> Option<FxState>`, `select_fx(ow, oh) -> Box<dyn FxRenderer>`, `render(...)`.

## fx_uniforms

Packs `FxState` into the `FxU` GPU uniform struct consumed by the FX shader, defining canonical numeric-id mappings for all effect modes plus the hue rotation Neon's second tube is drawn in. Key items: `FxU` (`repr(C)` bytemuck struct, including `color2`), `build_fx_u(state, ow, oh) -> FxU`, `style_id`, `spot_mode_id`, `video_mode_id`, `hue_shift(rgb, deg) -> [f32; 3]`, `MAX_HITS` and `NEON_HUE_SHIFT` constants.

## fx_gpu

GPU-backed `FxRenderer` that uploads the composited frame, runs the FX shader (`fx.wgsl` + `fx_clicks.wgsl`, concatenated into one module: spotlight, click effects, video FX), and reads processed pixels back into the caller's buffer. Key items: `GpuFx` struct, `GpuFx::new(ow, oh) -> Option<GpuFx>`, `GpuFx::apply(out, ow, oh, state)`.

## fx_gpu_pipeline

The wgpu objects `GpuFx` is built from, kept out of the renderer file: `build_pipeline` (bind-group layout + render pipeline, compiling the three `.wgsl` files as one module) and `r8_texture` (upload an R8 mask and return its view).

## fxdraw

CPU fallback `FxRenderer` that applies the full effect stack in layer order: video FX then spotlight then click effects. Key items: `CpuFx` (unit struct implementing `FxRenderer`), `CpuFx::apply(out, ow, oh, state)`.

## click

Pure data layer for click effects: computes live hit sets and the two timing curves EVERY click style shares (mirrored by `fx_clicks.wgsl` and by `src/editor/stage/fx/ripplePreview.ts`, all three pinning the same five sample points). Key items: `Hit` (screen-space click with normalized progress), `hits_at(events, et, life_ms)` - returns all live hits at event-time `et`; `ease_out(progress)` (ease-out cubic radii), `fade_alpha(progress, intensity)` (hold then smoothstep release), `ripple_radius(progress, r_max)`, `smoothstep(e0, e1, x)`.

## spot

The spotlight: which effect region is driving and at what alpha (`spotlight_sim`), the hold-key adapter over `hold` (`spotlight`), and the CPU pass that darkens everything outside the lit zone (`spotdraw`). See `spot/mod.md`.

## lens

The glass cursor material: the shapes and the timing math (`lens/mod.rs`), where they go each frame (`lens/build.rs`), the silhouettes the refraction is confined to (`lens/mask.rs`), and the CPU stand-in for `fx_lens.wgsl` (`lens/draw.rs`). See `lens/mod.md`.

## caption

Thin typed adapter over `hold::hold_alpha` for the spotlight effect. Key items: `hold_alpha(actions, et, fade_ms)` - returns spotlight strength 0..1 ramping around `SpotlightHoldStart`/`SpotlightHoldEnd` action pairs.

## videodraw

Applies full-frame video effects (CinematicDim, ScreenFocus, ColorPop, NebulaWash) in software onto the composited BGRA frame. Key items: `draw_video(out, ow, oh, v)` - CPU fallback for all `VideoFxMode` variants, all effects expressed as per-pixel multiplications or additions scaled by `v.alpha`.

## hold

Generic fade-in/hold/fade-out alpha ramp shared by spotlight and video FX, accepting predicate closures for start and end action kinds. Key items: `hold_alpha(actions, et, fade_ms, is_start, is_end) -> f32`.
