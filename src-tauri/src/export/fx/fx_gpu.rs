use wgpu::util::DeviceExt;
use crate::export::fx::fx_state::{FxRenderer, FxState};
use crate::export::fx::fx_uniforms::build_fx_u;
use crate::export::gpu::{align_up, FORMAT};

/// GPU FX renderer: uploads the composited frame, runs fx.wgsl (spotlight + clicks),
/// reads the result back. Selected when an adapter is available.
pub struct GpuFx {
    device: wgpu::Device, queue: wgpu::Queue, sampler: wgpu::Sampler,
    bind_layout: wgpu::BindGroupLayout, pipeline: wgpu::RenderPipeline,
    out_tex: wgpu::Texture, out_view: wgpu::TextureView, readback: wgpu::Buffer, padded_bpr: u32,
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
        ] });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fx"), source: wgpu::ShaderSource::Wgsl(include_str!("fx.wgsl").into()) });
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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("tcursor-fx"), required_features: wgpu::Features::empty(),
            required_limits: limits, memory_hints: wgpu::MemoryHints::default() }, None)).ok()?;
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
        Some(GpuFx { device, queue, sampler, bind_layout, pipeline, out_tex, out_view, readback, padded_bpr })
    }
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
        let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("fxbg"),
            layout: &g.bind_layout, entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&fv) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&g.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: ubuf.as_entire_binding() },
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
        let slice = g.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map fx readback"));
        g.device.poll(wgpu::Maintain::Wait);
        let unpadded = (ow * 4) as usize;
        {
            let data = slice.get_mapped_range();
            for row in 0..oh as usize {
                let src = row * g.padded_bpr as usize;
                let dst = row * unpadded;
                out[dst..dst + unpadded].copy_from_slice(&data[src..src + unpadded]);
            }
        }
        g.readback.unmap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::fx::fx_state::Spot;
    use crate::settings::model::{ClickFxStyle, SpotlightMode};

    #[test]
    fn spotlight_dims_corner_more_than_center() {
        let g = match GpuFx::new(64, 64) { Some(g) => g, None => return }; // skip without adapter
        let (w, h) = (64u32, 64u32);
        let mut out = vec![200u8; (w * h * 4) as usize];
        let st = FxState { style: ClickFxStyle::None, color: [0, 0, 0], intensity: 1.0, hits: vec![],
            spot: Some(Spot { cx: 32.0, cy: 32.0, dim: 0.7, radius_frac: 0.13, feather_frac: 0.10, alpha: 1.0,
                mode: SpotlightMode::Classic, tint: [0, 0, 0], t: 0.0,
                cam_rect: [0.0; 4], cam_radius: 0.0, dim_camera: true }), video: None };
        g.apply(&mut out, w, h, &st);
        assert!(out[0] < out[((32 * w + 32) * 4) as usize], "GPU spotlight: corner dimmer than center");
    }
}
