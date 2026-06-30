# src-tauri/src/export/mod.rs

The `export` module is the render pipeline that turns a raw recording plus an `edit.json` plan into the final `final.mp4`. The high-level flow is: decode raw BGRA frames from ffmpeg (`ffio`, `timeline`) -> resolve the virtual camera and per-frame scene geometry (`autozoom`, `camera`, `scene`, `layout`) -> composite background + screen panel + webcam panel in either software or on the GPU (`compositor`, `gpu`, `gpu_compositor`, `gpu_uniforms`) -> draw overlays on the composited frame (cursor via `cursor`, `cursorset`, `cursordraw`; click/spotlight/video FX via `fx_state`, `fxdraw`, `fx_gpu`, `clickdraw`, `spotdraw`, `videodraw`, `clickfx`, `caption`) -> encode and mux audio (`audio_mux`). Shared pure-data foundations span the whole stack: `types` holds every value type (colors, camera, geometry), `coordmap` provides all coordinate-space conversions, and `easing` supplies the interpolation curve used by camera animation and layout transitions. The edit doc is read in by `fromedit` and re-anchored into the active screen panel by `layout`; manual hold-zoom and auto-zoom regions are produced by `manual` and `autozoom` respectively. The main entry point is `exporter::export`, wired to the Tauri frontend by `run`.

## types

Shared pure-data types for the entire pipeline with no methods beyond `Default`. Key items: `Rgb` (24-bit color), `FramePoint` (signed integer pixel coordinate), `RectF` (float axis-aligned rectangle), `Camera` (zoom center + scale), `Easing` (interpolation curve variant), `ZoomConfig` (all click-zoom tunables), `ZoomRegion` (one resolved zoom event), `Background` (gradient/solid/image enum), `Layout` (output canvas and screen panel geometry), `OverlayShape`, `OverlayPos`, `OverlayLayout` (webcam overlay parameters).

## easing

Single stateless function mapping a normalized time value through a curve. Key items: `ease(e, t)` - clamps `t` to 0..1 and applies the curve selected by `Easing` (Linear, Smooth cubic, Spring).

## coordmap

All coordinate-space conversions used by every subsystem, pure with no state. Key items: `to_frame` (global screen -> frame-local), `inset_rect` (aspect-fitted screen panel rect in output pixels), `corner_radius` (clamped corner radius), `to_base` (frame pixel -> composited base), `to_panel` (frame pixel -> any panel rect), `crop` (zoom crop rectangle), `project` (base output pixel -> zoomed output pixel).

## autozoom

Generates automatic zoom regions from recorded mouse clicks and optional typing events. Key items: `generate(events, screen, cfg, typing, smart)` - returns `Vec<ZoomRegion>` with one region per detected activity burst; `extend_hold` - private helper that chains consecutive activities within an idle gap.

## camera

Stateful virtual camera that exponentially damps zoom scale and pan center toward per-frame setpoints derived from zoom regions and cursor position. Key items: `CameraSim` struct, `CameraSim::new(frame_w, frame_h)`, `CameraSim::step(t_ms, cursor, regions, cfg)` - returns a `Camera` for the frame.

## clickfx

Pure data layer for click effects: computes live hit sets and per-hit animation scalars. Key items: `Hit` (screen-space click with normalized progress), `hits_at(events, et, life_ms)` - returns all live hits at event-time `et`; `ripple_radius(progress, r_max)`, `fade_alpha(progress, intensity)`.

## fxdraw

CPU fallback `FxRenderer` that applies the full effect stack in layer order: video FX then spotlight then click effects. Key items: `CpuFx` (unit struct implementing `FxRenderer`), `CpuFx::apply(out, ow, oh, state)`.

## clickdraw

Renders all click effect styles (Ripple, Pulse, Glow, Neon, Shockwave, Particles) onto a BGRA frame. Key items: `draw_clicks(out, ow, oh, state)` - dispatches per-hit draws from `FxState`, all sizes as fractions of `oh`.

## spotdraw

Renders the spotlight effect in six modes (Classic, Breathing, Vignette, Blur, Nebula, Halo) by darkening pixels outside the lit zone. Key items: `draw_spot(out, ow, oh, s)` - modifies `out` in-place, skips the double loop entirely when `s.dim * s.alpha <= 0`.

## caption

Renders keyboard-shortcut chord captions using embedded Inter SemiBold, centered in the bottom band of the frame with a drop shadow. Key items: `overlay(out, ow, oh, actions, keys, et, enabled)` - per-frame entry point; `caption_at(actions, keys, et, life_ms)` - returns active caption text and fade alpha; `draw_caption(out, ow, oh, text, alpha)`.

## spotlight

Thin typed adapter over `hold::hold_alpha` for the spotlight effect. Key items: `hold_alpha(actions, et, fade_ms)` - returns spotlight strength 0..1 ramping around `SpotlightHoldStart`/`SpotlightHoldEnd` action pairs.

## background

Rasterizes the export background (solid, gradient at any angle, or image stub) into a BGRA pixel buffer. Key items: `render(bg, w, h)` - returns `Vec<u8>` of `w*h*4` BGRA bytes, always deterministic.

## compositor

Defines the `Compositor` trait shared by CPU and GPU implementations, and provides the software `CpuCompositor`. Key items: `Compositor` trait with `composite(screen, sw, sh, webcam, cam, bg, layout, scene) -> Vec<u8>`; `CpuCompositor` (zero-state, always available).

## gpu

wgpu device/pipeline initialization, GPU availability probe, and texture upload helper. Key items: `Gpu` struct (device, queue, pipeline, output texture, readback buffer), `Gpu::new(out_w, out_h) -> Option<Gpu>`, `Gpu::upload_tex`, `gpu_available() -> bool`, `FORMAT` constant (`Bgra8Unorm`), `align_up`.

## gpu_compositor

GPU-accelerated compositor implementing the `Compositor` trait via wgpu + a WGSL compositing shader. Key items: `GpuCompositor` (holds `Gpu` plus a `OnceLock` background texture), `GpuCompositor::new(out_w, out_h) -> Option<GpuCompositor>`, `GpuCompositor::composite`.

## gpu_uniforms

Defines the GPU uniform buffer layout and the builder that populates it from per-frame scene, camera, and layout state. Key items: `Uniforms` (`repr(C)` bytemuck-castable struct with UV panel bounds, zoom center, radii, alphas, sizes), `build_uniforms(scene, cam, layout, has_webcam) -> Uniforms`.

## ffio

FFmpeg and ffprobe spawn helpers, raw BGRA frame reader, and bundled-image decode/crop utilities. Key items: `RawDecoder` (spawned ffmpeg subprocess, `spawn` + `read_frame`), `probe_dims`, `probe_duration`, `probe_frame_count`, `decode_image`, `decode_cursor`, `crop_to_alpha`, `png_dims`.

## audio_mux

Muxes the encoded silent video with microphone and/or system audio into `final.mp4`, handling all four audio combinations and applying per-track A/V sync offsets. Key items: `mux(tmp, paths, mic_shift_ms, sys_shift_ms) -> Result<()>`.

## timeline

Builds the per-frame timestamp vector from the sync log or a synthesized uniform fallback, plus audio track start offsets. Key items: `Timeline` struct (`frames: Vec<u64>`, `events_ms`, `mic_ms`, `system_ms`), `build_timeline(paths, log, fps) -> Timeline`.

## cursor

Stateful cursor position tracker that interpolates between mouse samples and applies an exponential low-pass filter to suppress jitter. Key items: `Cursor` struct (owns its event log), `Cursor::new`, `Cursor::at(t_ms) -> FramePoint`, `Cursor::events`.

## exporter

Top-level export orchestrator: calls `FrameRenderer::new`, spawns decoders and the encoder thread, drives the per-frame loop via `step_camera` + `composite_at`, and muxes audio. Key items: `export(paths, fps, on_progress) -> Result<()>` - single public entry point; `read_webcam(dec, buf, size)`.

## render

Reusable per-frame renderer extracted from `exporter.rs`; owns all compositing state except raw decoders and the encoder sink. Key items: `OUT_FPS` constant (60); `RenderMeta` struct (decoder setup info returned by `new`); `FramePose` struct (resolved camera + scene for one frame); `FrameRenderer` struct, `FrameRenderer::new(paths, layout, fps) -> Result<(Self, RenderMeta)>`, `FrameRenderer::step_camera(t) -> FramePose`, `FrameRenderer::composite_at(pose, screen, webcam) -> Vec<u8>`; `select_compositor(layout) -> Box<dyn Compositor>`.

## run

Thin Tauri command adapter that launches the export on a background thread and bridges results to the frontend as events. Key items: `run_export(app, folder)` - fire-and-forget; emits `export-progress`, `export-done`, and `export-error`.

## scene

Defines two-panel scene geometry, resolves `LayoutId` presets into output-pixel rectangles and radii, and provides cross-dissolve interpolation and zoom-driven camera shrink. Key items: `Panel` (rect + radius + alpha), `Scene` (screen + camera panels), `Scene::lerp(a, b, t)`, `shrink_camera(panel, scale, target_scale, min)`, `resolve(id, layout, overlay, sw, sh) -> Scene`.

## layout

Resolves the active `Scene` at any video timestamp from the `SetLayout` action track, cross-fading between presets, and re-anchors zoom regions into the active screen panel. Key items: `LayoutTrack` struct, `LayoutTrack::new(actions, app, ow, oh, sw, sh, transition_ms)`, `LayoutTrack::scene_at(t_ms) -> Scene`, `anchor_regions(raw, track, sw, sh) -> Vec<ZoomRegion>`.

## manual

Converts `ZoomHoldStart`/`ZoomHoldEnd` action pairs into `ZoomRegion` values anchored at the cursor position when the key was pressed. Key items: `from_actions(actions, events, screen, cfg) -> Vec<ZoomRegion>`.

## fromedit

Reconstructs `ZoomRegion` and `SetLayout` action tracks from a persisted `EditDoc`, the inverse of `edit::seed`. Key items: `regions_from_doc(doc, sw, sh) -> Vec<ZoomRegion>`, `layout_segs_from_doc(doc) -> Option<Vec<ActionEvent>>`.

## hold

Generic fade-in/hold/fade-out alpha ramp shared by spotlight and video FX, accepting predicate closures for start and end action kinds. Key items: `hold_alpha(actions, et, fade_ms, is_start, is_end) -> f32`.

## fx_state

Renderer-agnostic per-frame FX data model, the `FxRenderer` trait, the renderer factory, and the per-frame render entry point. Key items: `FxHit`, `Spot`, `VideoFx`, `FxState`, `FxRenderer` trait, `fx_state_at(fx, events, actions, scene, cam, cur, sw, sh, ow, oh, et) -> Option<FxState>`, `select_fx(ow, oh) -> Box<dyn FxRenderer>`, `render(...)`.

## fx_uniforms

Packs `FxState` into the `FxU` GPU uniform struct consumed by `fx.wgsl`, defining canonical numeric-id mappings for all effect modes. Key items: `FxU` (`repr(C)` bytemuck struct), `build_fx_u(state, ow, oh) -> FxU`, `style_id`, `spot_mode_id`, `video_mode_id`, `MAX_HITS` constant.

## fx_gpu

GPU-backed `FxRenderer` that uploads the composited frame, runs `fx.wgsl` (spotlight, click effects, video FX), and reads processed pixels back into the caller's buffer. Key items: `GpuFx` struct, `GpuFx::new(ow, oh) -> Option<GpuFx>`, `GpuFx::apply(out, ow, oh, state)`.

## videodraw

Applies full-frame video effects (CinematicDim, ScreenFocus, ColorPop, NebulaWash) in software onto the composited BGRA frame. Key items: `draw_video(out, ow, oh, v)` - CPU fallback for all `VideoFxMode` variants, all effects expressed as per-pixel multiplications or additions scaled by `v.alpha`.

## cursordraw

CPU rasterizer for the Enhanced synthetic cursor: sprite placement, click-bounce scale animation, and motion trail blending, clipped to the screen panel. Key items: `CursorSprite` struct (content-tight BGRA sprite with hotspot fractions), `decode_sprite(png, hot) -> Option<CursorSprite>`, `bounce_scale(click_ms, t_ms, enabled, intensity) -> f32`, `draw_cursor(out, ow, oh, spr, pos, recent, size_px, blur, bounce, clip)`, `apply_enhanced(...)`.

## preview

Single-frame preview engine: renders one composited frame at an arbitrary scrub position from `edit.json`, reusing `FrameRenderer` so the preview is byte-faithful to the export. Key items: `render_preview(paths, time_ms, out_w, out_h) -> Result<Vec<u8>>` (fast-forwards step_camera, seek-decodes screen + webcam, composites, PNG-encodes via ffmpeg); `preview_frame(folder, time_ms) -> Result<String, String>` (Tauri command wrapping render_preview at 1280x720, returns PNG data URL).

## cursorset

Manages the per-type cursor sprite set: decodes each shape once at prep time, inverts RGB for dark themes, and dispatches per-frame draw calls with panel-proportional sizing. Key items: `SPRITES` compile-time table, `CursorPrep` struct (`set`, `track`, `click_ms`, `recent`), `prep(cursor, events, track, dark) -> Option<CursorPrep>`, `sprite_for(set, track, ev_t)`, `draw(cp, out, ow, oh, cur, cam, screen, inset_w, ev_t, c)`, `invert_rgb`.
