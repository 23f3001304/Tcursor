use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use crate::export::fx::fx_state::{FxRenderer, FxState};
use crate::export::fx::fx_uniforms::build_fx_u;
use crate::export::gpu::{align_up, FORMAT};

/// GPU FX renderer: uploads the composited frame, runs fx.wgsl (spotlight + clicks + the glass
/// cursor lens), reads the result back. Selected when an adapter is available.
pub struct GpuFx {
    device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>, sampler: wgpu::Sampler,
    bind_layout: wgpu::BindGroupLayout, pipeline: wgpu::RenderPipeline,
    out_tex: wgpu::Texture, out_view: wgpu::TextureView, readback: wgpu::Buffer, padded_bpr: u32,
    /// The cursor lens mask (`fx_lens::LensMask`), uploaded once per (pack, kind) and kept until a
    /// different one is asked for. One entry is enough: a frame draws ONE cursor, and a shape
    /// change re-uploads 16 KB. Behind a `Mutex` because `FxRenderer::apply` takes `&self`.
    mask: Mutex<Option<(u64, wgpu::TextureView)>>,
    /// The 1x1 opaque texture bound when no lens is live - the layout always needs a binding.
    blank: wgpu::TextureView,
}

fn build_pipeline(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::RenderPipeline) {
    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fxbind"), entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
        ] });
    // ONE module from THREE files: fx.wgsl (uniforms, spotlight, video fx), fx_clicks.wgsl (every
    // click style), fx_lens.wgsl (the glass cursor material). Split purely for the 200-line budget;
    // WGSL resolves module-scope names out of order, so fs_main can call `clicks()` and `lens_fx()`
    // from the halves appended after it.
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fx"), source: wgpu::ShaderSource::Wgsl(concat!(
            include_str!("fx.wgsl"), "\n", include_str!("fx_clicks.wgsl"), "\n",
            include_str!("fx_lens.wgsl")).into()) });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("fxpl"), bind_group_layouts: &[&bind_layout], push_constant_ranges: &[] });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fxpipe"), layout: Some(&pl),
        vertex: wgpu::VertexState { module: &shader, entry_point: "vs_main", buffers: &[],
            compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: None,
                write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default() }),
        primitive: wgpu::PrimitiveState::default(), depth_stencil: None,
        multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
    (bind_layout, pipeline)
}

impl GpuFx {
    pub fn new(ow: u32, oh: u32) -> Option<GpuFx> {
        // Shares the process-global device with the compositor (see `gpu::shared_device`) instead of
        // creating a SECOND wgpu device per renderer build - removing half the device-creation cost
        // that made aspect changes lag.
        let (device, queue) = crate::export::gpu::shared_device()?;
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear,
            ..Default::default() });
        let (bind_layout, pipeline) = build_pipeline(&device);
        let out_tex = device.create_texture(&wgpu::TextureDescriptor { label: Some("fxout"),
            size: wgpu::Extent3d { width: ow, height: oh, depth_or_array_layers: 1 }, mip_level_count: 1,
            sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
        let out_view = out_tex.create_view(&Default::default());
        let padded_bpr = align_up(ow * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor { label: Some("fxread"),
            size: (padded_bpr * oh) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false });
        let blank = r8_texture(&device, &queue, 1, 1, &[255]);
        Some(GpuFx { device, queue, sampler, bind_layout, pipeline, out_tex, out_view, readback,
            padded_bpr, mask: Mutex::new(None), blank })
    }
}

/// Upload `a` as a single-channel R8 texture and return its view. `write_texture` (not a buffer
/// copy) so an arbitrary sprite width needs no 256-byte row padding.
fn r8_texture(device: &wgpu::Device, queue: &wgpu::Queue, w: u32, h: u32, a: &[u8]) -> wgpu::TextureView {
    let size = wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 };
    let tex = device.create_texture(&wgpu::TextureDescriptor { label: Some("fxmask"), size,
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
    queue.write_texture(tex.as_image_copy(), a,
        wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(w.max(1)), rows_per_image: Some(h.max(1)) },
        size);
    tex.create_view(&Default::default())
}

impl FxRenderer for GpuFx {
    fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState) {
        let g = &self;
        let frame = g.device.create_texture_with_data(&g.queue, &wgpu::TextureDescriptor {
            label: Some("fxframe"), size: wgpu::Extent3d { width: ow, height: oh, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] },
            wgpu::util::TextureDataOrder::LayerMajor, out);
        let fv = frame.create_view(&Default::default());
        let u = build_fx_u(state, ow, oh);
        let ubuf = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fxu"), contents: bytemuck::bytes_of(&u), usage: wgpu::BufferUsages::UNIFORM });
        // The lens mask, re-uploaded only when the (pack, kind) behind it actually changed. A
        // poisoned lock is recovered rather than propagated: this runs on a Tauri command thread
        // for the editor preview, where a panic takes out the overlay (see `readback_into`).
        let glass = state.lens.as_ref().and_then(|l| l.glass.as_ref());
        let mut cache = g.mask.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(gl) = glass {
            if cache.as_ref().map(|(k, _)| *k) != Some(gl.mask.key) {
                *cache = Some((gl.mask.key, r8_texture(&g.device, &g.queue, gl.mask.w, gl.mask.h, &gl.mask.a)));
            }
        }
        let mask = match (glass, cache.as_ref()) { (Some(_), Some((_, v))) => v, _ => &g.blank };
        let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("fxbg"),
            layout: &g.bind_layout, entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&g.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: ubuf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(mask) },
            ] });
        let mut enc = g.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("fxpass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &g.out_view,
                    resolve_target: None, ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
            rp.set_pipeline(&g.pipeline);
            rp.set_bind_group(0, &bind, &[]);
            rp.draw(0..3, 0..1);
        }
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture { texture: &g.out_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All },
            wgpu::ImageCopyBuffer { buffer: &g.readback, layout: wgpu::ImageDataLayout { offset: 0,
                bytes_per_row: Some(g.padded_bpr), rows_per_image: Some(oh) } },
            wgpu::Extent3d { width: ow, height: oh, depth_or_array_layers: 1 });
        g.queue.submit(Some(enc.finish()));
        if !g.readback_into(out, ow, oh) {
            // The GPU result is unreachable, but `out` still holds the composited frame we
            // uploaded (readback failed before writing a byte), so the CPU path can just redo the
            // effect. Warn once - a repeating device fault would otherwise spam every frame.
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED.call_once(|| eprintln!("fx: GPU readback failed; falling back to the CPU renderer"));
            crate::export::fx::fxdraw::CpuFx.apply(out, ow, oh, state);
        }
    }
}

impl GpuFx {
    /// Map the readback buffer and copy the rendered frame into `out`, dropping the row padding.
    /// `false` if the map failed. NEVER panics: this runs on the caller's thread, which for the
    /// editor preview is a Tauri command thread (see `preview_fx::with_fx`), where a panic would
    /// take out the overlay rather than one frame of it.
    fn readback_into(&self, out: &mut [u8], ow: u32, oh: u32) -> bool {
        let slice = self.readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r.is_ok()); });
        self.device.poll(wgpu::Maintain::Wait); // Wait -> the callback has fired by now
        if !matches!(rx.try_recv(), Ok(true)) { return false; }
        let unpadded = (ow * 4) as usize;
        {
            let data = slice.get_mapped_range();
            for row in 0..oh as usize {
                let src = row * self.padded_bpr as usize;
                let dst = row * unpadded;
                out[dst..dst + unpadded].copy_from_slice(&data[src..src + unpadded]);
            }
        }
        self.readback.unmap();
        true
    }
}

#[cfg(test)]
#[path = "fx_gpu_tests.rs"]
mod tests;
