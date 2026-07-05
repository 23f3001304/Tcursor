# src-tauri/src/export/mod.rs

The `export` module is the render pipeline that turns a raw recording plus an `edit.json` plan into the final `final.mp4`. The high-level flow is: decode raw BGRA frames from ffmpeg (`ffio`, `timeline`) -> resolve the virtual camera and per-frame scene geometry (`autozoom`, `camera`, `scene`, `layout`) -> composite background + screen panel + webcam panel in either software or on the GPU (`compositor`, `gpu`, `gpu_compositor`, `gpu_uniforms`) -> draw overlays on the composited frame (cursor via `cursor`, `cursorset`, `cursordraw`; click/spotlight/video FX via `fx_state`, `fxdraw`, `fx_gpu`, `clickdraw`, `spotdraw`, `videodraw`, `clickfx`, `caption`) -> encode and mux audio (`audio_mux`). Shared pure-data foundations span the whole stack: `types` holds every value type (colors, camera, geometry), `coordmap` provides all coordinate-space conversions, and `easing` supplies the interpolation curve used by camera animation and layout transitions. The edit doc is read in by `fromedit` and re-anchored into the active screen panel by `layout`; manual hold-zoom and auto-zoom regions are produced by `manual` and `autozoom` respectively. The main entry point is `exporter::export`, wired to the Tauri frontend by `run`.

## types

Shared pure-data types for the entire pipeline with no methods beyond `Default`. Key items: `Rgb` (24-bit color), `FramePoint` (signed integer pixel coordinate), `RectF` (float axis-aligned rectangle), `Camera` (zoom center + scale), `Easing` (interpolation curve variant), `ZoomConfig` (all click-zoom tunables), `ZoomRegion` (one resolved zoom event), `Background` (gradient/solid/image enum), `Layout` (output canvas and screen panel geometry), `OverlayShape`, `OverlayPos`, `OverlayLayout` (webcam overlay parameters).

## easing

Single stateless function mapping a normalized time value through a curve. Key items: `ease(e, t)` - clamps `t` to 0..1 and applies the curve selected by `Easing` (Linear, Smooth cubic, Spring).

## coordmap

All coordinate-space conversions used by every subsystem, pure with no state. Key items: `to_frame` (global screen -> frame-local), `inset_rect` (aspect-fitted screen panel rect in output pixels), `corner_radius` (clamped corner radius), `to_base` (frame pixel -> composited base), `to_panel` (frame pixel -> any panel rect), `crop` (zoom crop rectangle), `project` (base output pixel -> zoomed output pixel).

## camera

Stateful virtual camera that exponentially damps zoom scale and pan center toward per-frame setpoints derived from zoom regions and cursor position. Key items: `CameraSim` struct, `CameraSim::new(frame_w, frame_h)`, `CameraSim::step(t_ms, cursor, regions, cfg)` - returns a `Camera` for the frame.

## gpu

wgpu device/pipeline initialization, GPU availability probe, and texture upload helper. Key items: `Gpu` struct (device, queue, pipeline, output texture, readback buffer), `Gpu::new(out_w, out_h) -> Option<Gpu>`, `Gpu::upload_tex`, `gpu_available() -> bool`, `FORMAT` constant (`Bgra8Unorm`), `align_up`.

## cursor

Stateful cursor position tracker that interpolates between mouse samples and applies an exponential low-pass filter to suppress jitter. Key items: `Cursor` struct (owns its event log), `Cursor::new`, `Cursor::at(t_ms) -> FramePoint`, `Cursor::events`.

## render

Reusable per-frame renderer extracted from `exporter.rs`; owns all compositing state except raw decoders and the encoder sink. Key items: `OUT_FPS` constant (60); `RenderMeta` struct (decoder setup info returned by `new`); `FramePose` struct (resolved camera + scene for one frame); `FrameRenderer` struct, `FrameRenderer::new(paths, layout, fps) -> Result<(Self, RenderMeta)>`, `FrameRenderer::step_camera(t) -> FramePose`, `FrameRenderer::composite_at(pose, screen, webcam, out: &mut Vec<u8>)`; `select_compositor(layout) -> Box<dyn Compositor>`.

## scene

Defines two-panel scene geometry, resolves `LayoutId` presets into output-pixel rectangles and radii, and provides cross-dissolve interpolation and zoom-driven camera shrink. Key items: `Panel` (rect + radius + alpha), `Scene` (screen + camera panels), `Scene::lerp(a, b, t)`, `shrink_camera(panel, scale, target_scale, min)`, `resolve(id, layout, overlay, sw, sh) -> Scene`.

## preview

Single-frame preview engine: renders one composited frame at an arbitrary scrub position from `edit.json`, reusing `FrameRenderer` so the preview is byte-faithful to the export. Key items: `render_preview(paths, time_ms, out_w, out_h) -> Result<Vec<u8>>` (fast-forwards step_camera, seek-decodes screen + webcam, composites, PNG-encodes via ffmpeg); `preview_frame(folder, time_ms) -> Result<String, String>` (Tauri command wrapping render_preview at 1280x720, returns PNG data URL).
