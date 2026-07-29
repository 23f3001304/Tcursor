// Texture + bind-group construction for `GpuCompositor::composite_into`, split out
// so gpu_compositor.rs stays under the size limit. Pure builder: identical to the
// inline block it replaces, no behavior change.
use wgpu::util::DeviceExt;
use crate::export::gpu::gpu_uniforms::Uniforms;
use crate::export::gpu::Gpu;
use super::CompositorResources;

/// Build the screen/webcam/bg textures, the uniform buffer, and the bind group for one
/// `(sw, sh, ww, wh)` size combination. Called only when `composite_into` detects a
/// size change (or on the first frame) - `bg_uploaded` always starts `false` so the
/// caller re-uploads the background into the freshly created `bg_tex`.
pub(super) fn build_resources(
    g: &Gpu, sw: u32, sh: u32, ww: u32, wh: u32, ow: u32, oh: u32, u: &Uniforms,
) -> CompositorResources {
    let screen_y_tex = g.create_tex_fmt("screen_y", sw, sh, wgpu::TextureFormat::R8Unorm);
    let screen_uv_tex = g.create_tex_fmt("screen_uv", (sw / 2).max(1), (sh / 2).max(1), wgpu::TextureFormat::Rg8Unorm);
    let webcam_tex = g.create_tex("webcam", ww, wh);
    let bg_tex = g.create_tex("bg", ow, oh);
    let ubuf = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("uniforms"),
        contents: bytemuck::bytes_of(u),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let yv = screen_y_tex.create_view(&Default::default());
    let uvv = screen_uv_tex.create_view(&Default::default());
    let bv = bg_tex.create_view(&Default::default());
    let wv = webcam_tex.create_view(&Default::default());
    let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind"),
        layout: &g.bind_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&bv) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&yv) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&uvv) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&wv) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&g.sampler) },
            wgpu::BindGroupEntry { binding: 5, resource: ubuf.as_entire_binding() },
        ],
    });
    CompositorResources { sw, sh, ww, wh, screen_y_tex, screen_uv_tex, webcam_tex, bg_tex, ubuf, bind, bg_uploaded: false }
}
