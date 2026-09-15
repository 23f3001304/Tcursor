use crate::export::gpu::compositor::Compositor;
use crate::export::gpu::gpu_uniforms::{build_uniforms, Uniforms};
use crate::export::gpu::Gpu;
use crate::export::scene::Scene;
use crate::export::types::{Camera, Layout};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use wgpu::util::DeviceExt;

struct CompositorResources {
    sw: u32,
    sh: u32,
    ww: u32,
    wh: u32,
    screen_y_tex: wgpu::Texture,
    screen_uv_tex: wgpu::Texture,
    webcam_tex: wgpu::Texture,
    bg_tex: wgpu::Texture,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    bg_key: Option<u64>,
}

pub struct GpuCompositor {
    gpu: Gpu,
    out_w: u32,
    out_h: u32,
    res: Mutex<Option<CompositorResources>>,
    bg_dynamic: AtomicBool,
}

impl GpuCompositor {
    pub fn new(out_w: u32, out_h: u32) -> Option<GpuCompositor> {
        Some(GpuCompositor {
            gpu: Gpu::new(out_w, out_h)?,
            out_w,
            out_h,
            res: Mutex::new(None),
            bg_dynamic: AtomicBool::new(false),
        })
    }
}

impl Compositor for GpuCompositor {
    fn composite_into(
        &self,
        screen: &[u8],
        sw: u32,
        sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera,
        bg: &[u8],
        layout: &Layout,
        scene: &Scene,
        out: &mut Vec<u8>,
    ) {
        let (ow, oh) = (self.out_w, self.out_h);
        let g = &self.gpu;
        let (wc_data, ww, wh) = webcam.unwrap_or((&[0u8; 4], 1, 1));
        let u = build_uniforms(scene, cam, layout, webcam.map(|(_, w, h)| (w, h)), (sw, sh));
        let dynamic = self.bg_dynamic.load(Ordering::Relaxed);
        let key = (!dynamic).then(|| bg_key(bg));

        let mut lock = self.res.lock().unwrap();
        let rebuild = match lock.as_ref() {
            Some(r) => r.sw != sw || r.sh != sh || r.ww != ww || r.wh != wh,
            None => true,
        };

        if rebuild {
            *lock = Some(build_resources(g, sw, sh, ww, wh, ow, oh, &u));
        }

        let r = lock.as_mut().unwrap();
        let y_size = (sw * sh) as usize;
        g.update_tex_bpp(&r.screen_y_tex, &screen[..y_size], sw, sh, 1);
        g.update_tex_bpp(&r.screen_uv_tex, &screen[y_size..], sw / 2, sh / 2, 2);
        g.update_tex(&r.webcam_tex, wc_data, ww, wh);
        if should_upload(r.bg_key != key, dynamic) {
            g.update_tex(&r.bg_tex, bg, ow, oh);
            r.bg_key = key;
        }
        g.queue.write_buffer(&r.ubuf, 0, bytemuck::bytes_of(&u));

        let mut enc = g.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &g.out_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&g.pipeline);
            rp.set_bind_group(0, &r.bind, &[]);
            rp.draw(0..3, 0..1);
        }
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &g.out_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &g.readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(g.padded_bpr),
                    rows_per_image: Some(oh),
                },
            },
            wgpu::Extent3d {
                width: ow,
                height: oh,
                depth_or_array_layers: 1,
            },
        );
        g.queue.submit(Some(enc.finish()));

        let slice = g.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map readback"));
        g.device.poll(wgpu::Maintain::Wait);

        let unpadded = (ow * 4) as usize;
        let padded = g.padded_bpr as usize;
        out.clear();
        {
            let data = slice.get_mapped_range();
            if padded == unpadded {
                out.extend_from_slice(&data[..unpadded * oh as usize]);
            } else {
                out.resize(unpadded * oh as usize, 0);
                for row in 0..oh as usize {
                    out[row * unpadded..row * unpadded + unpadded]
                        .copy_from_slice(&data[row * padded..row * padded + unpadded]);
                }
            }
        }
        g.readback.unmap();
    }

    fn set_bg_dynamic(&self, dynamic: bool) {
        self.bg_dynamic.store(dynamic, Ordering::Relaxed);
    }
}

fn bg_key(bg: &[u8]) -> u64 {
    const FNV: u64 = 0x100000001b3;
    let mut h = (0xcbf29ce484222325u64 ^ bg.len() as u64).wrapping_mul(FNV);
    let words = bg.len() / 4;
    for i in (0..words).step_by((words / 4096).max(1)) {
        let w = u32::from_le_bytes([bg[i * 4], bg[i * 4 + 1], bg[i * 4 + 2], bg[i * 4 + 3]]);
        h = (h ^ w as u64).wrapping_mul(FNV);
    }
    h
}

fn should_upload(key_changed: bool, dynamic: bool) -> bool {
    key_changed || dynamic
}

fn build_resources(
    g: &Gpu,
    sw: u32,
    sh: u32,
    ww: u32,
    wh: u32,
    ow: u32,
    oh: u32,
    u: &Uniforms,
) -> CompositorResources {
    let screen_y_tex = g.create_tex_fmt("screen_y", sw, sh, wgpu::TextureFormat::R8Unorm);
    let screen_uv_tex = g.create_tex_fmt(
        "screen_uv",
        (sw / 2).max(1),
        (sh / 2).max(1),
        wgpu::TextureFormat::Rg8Unorm,
    );
    let webcam_tex = g.create_tex("webcam", ww, wh);
    let bg_tex = g.create_tex("bg", ow, oh);
    let ubuf = g
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&bv),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&yv),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&uvv),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&wv),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&g.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: ubuf.as_entire_binding(),
            },
        ],
    });
    CompositorResources {
        sw,
        sh,
        ww,
        wh,
        screen_y_tex,
        screen_uv_tex,
        webcam_tex,
        bg_tex,
        ubuf,
        bind,
        bg_key: None,
    }
}

#[cfg(test)]
#[path = "gpu_compositor_tests.rs"]
mod tests;
