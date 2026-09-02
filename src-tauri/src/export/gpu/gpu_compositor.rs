use std::sync::Mutex;
use crate::export::gpu::compositor::Compositor;
use crate::export::gpu::Gpu;
use crate::export::gpu::gpu_uniforms::build_uniforms;
use crate::export::scene::Scene;
use crate::export::types::{Camera, Layout};

#[path = "gpu_compositor_tex.rs"]
mod gpu_compositor_tex;

struct CompositorResources {
    sw: u32,
    sh: u32,
    ww: u32,
    wh: u32,
    screen_y_tex: wgpu::Texture,   // nv12 Y plane (R8, sw x sh)
    screen_uv_tex: wgpu::Texture,  // nv12 interleaved UV (Rg8, sw/2 x sh/2)
    webcam_tex: wgpu::Texture,
    bg_tex: wgpu::Texture,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    bg_key: Option<u64>, // content key of the uploaded background (None = nothing uploaded yet)
}

pub struct GpuCompositor {
    gpu: Gpu,
    out_w: u32,
    out_h: u32,
    res: Mutex<Option<CompositorResources>>,
}

impl GpuCompositor {
    pub fn new(out_w: u32, out_h: u32) -> Option<GpuCompositor> {
        Some(GpuCompositor { gpu: Gpu::new(out_w, out_h)?, out_w, out_h, res: Mutex::new(None) })
    }
}

impl Compositor for GpuCompositor {
    fn composite_into(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        scene: &Scene,
        out: &mut Vec<u8>,
    ) {
        let (ow, oh) = (self.out_w, self.out_h);
        // No raw-copy fast-path here: `screen` is nv12, so it always needs the shader's color
        // convert (a raw passthrough would emit nv12 bytes as bgra). The GPU pass is cheap anyway.
        let g = &self.gpu;
        let (wc_data, ww, wh) = webcam.unwrap_or((&[0u8; 4], 1, 1));
        let u = build_uniforms(scene, cam, layout, webcam.map(|(_, w, h)| (w, h)));
        // Hashed BEFORE the lock: it is a strided read over the whole ~8 MB background (~0.1-0.3 ms,
        // cache-miss bound), and nothing about it needs the cached resources.
        let key = gpu_compositor_tex::bg_key(bg);

        let mut lock = self.res.lock().unwrap();
        let rebuild = match lock.as_ref() {
            Some(r) => r.sw != sw || r.sh != sh || r.ww != ww || r.wh != wh,
            None => true,
        };

        if rebuild {
            *lock = Some(gpu_compositor_tex::build_resources(g, sw, sh, ww, wh, ow, oh, &u));
        }

        let r = lock.as_mut().unwrap();
        // Upload the nv12 screen: Y plane (R8, full res) then interleaved UV (Rg8, half res).
        let y_size = (sw * sh) as usize;
        g.update_tex_bpp(&r.screen_y_tex, &screen[..y_size], sw, sh, 1);
        g.update_tex_bpp(&r.screen_uv_tex, &screen[y_size..], sw / 2, sh / 2, 2);
        g.update_tex(&r.webcam_tex, wc_data, ww, wh);
        // Re-upload whenever the background's CONTENT changed, not just its dimensions: an edit
        // (colour/blur/kind) rebuilds `bg` at the same size, and a dimension-only check threw
        // every rebuilt buffer away, so the warm preview kept showing the old background.
        if r.bg_key != Some(key) {
            g.update_tex(&r.bg_tex, bg, ow, oh);
            r.bg_key = Some(key);
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
                texture: &g.out_tex, mip_level: 0,
                origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &g.readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(g.padded_bpr),
                    rows_per_image: Some(oh),
                },
            },
            wgpu::Extent3d { width: ow, height: oh, depth_or_array_layers: 1 },
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
                // No row padding (true whenever `ow*4` is 256-aligned, i.e. every standard output
                // width) - one contiguous copy instead of `oh` bounds-checked row copies, which is
                // a real per-frame saving in debug where each row slice is bounds-checked.
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
}

#[cfg(test)]
#[path = "gpu_compositor_tests.rs"]
mod tests;
