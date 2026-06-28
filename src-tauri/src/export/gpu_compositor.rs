use wgpu::util::DeviceExt;
use crate::export::compositor::Compositor;
use crate::export::gpu::Gpu;
use crate::export::gpu_uniforms::build_uniforms;
use crate::export::scene::Scene;
use crate::export::types::{Camera, Layout};

pub struct GpuCompositor {
    gpu: Gpu,
    out_w: u32,
    out_h: u32,
    bg_tex: std::sync::OnceLock<wgpu::Texture>,
}

impl GpuCompositor {
    pub fn new(out_w: u32, out_h: u32) -> Option<GpuCompositor> {
        Some(GpuCompositor { gpu: Gpu::new(out_w, out_h)?, out_w, out_h, bg_tex: std::sync::OnceLock::new() })
    }
}

impl Compositor for GpuCompositor {
    fn composite(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        scene: &Scene,
    ) -> Vec<u8> {
        let g = &self.gpu;
        let (ow, oh) = (self.out_w, self.out_h);

        let screen_tex = g.upload_tex("screen", screen, sw, sh);
        // The background is constant for the whole export -- upload it once, reuse it.
        let bg_tex = self.bg_tex.get_or_init(|| g.upload_tex("bg", bg, ow, oh));
        let (wc_data, ww, wh) = webcam.unwrap_or((&[0u8; 4], 1, 1));
        let wc_tex = g.upload_tex("webcam", wc_data, ww, wh);

        let sv = screen_tex.create_view(&Default::default());
        let bv = bg_tex.create_view(&Default::default());
        let wv = wc_tex.create_view(&Default::default());

        let u = build_uniforms(scene, cam, layout, webcam.is_some());
        let ubuf = g.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&u),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind"),
            layout: &g.bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&bv) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&sv) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&wv) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&g.sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: ubuf.as_entire_binding() },
            ],
        });

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
            rp.set_bind_group(0, &bind, &[]);
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
        let mut out = vec![0u8; unpadded * oh as usize];
        {
            let data = slice.get_mapped_range();
            for row in 0..oh as usize {
                let src = row * g.padded_bpr as usize;
                let dst = row * unpadded;
                out[dst..dst + unpadded].copy_from_slice(&data[src..src + unpadded]);
            }
        }
        g.readback.unmap();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::scene::{Panel, Scene};
    use crate::export::types::{Camera, Layout, RectF};

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for c in v.chunks_mut(4) { c.copy_from_slice(&px); } v
    }

    #[test]
    fn screen_panel_composites_onto_background() {
        let c = match GpuCompositor::new(8, 8) { Some(c) => c, None => return };
        let screen = solid(4, 4, [0, 0, 255, 255]);
        let bg = solid(8, 8, [255, 0, 0, 255]);
        let layout = Layout { out_w: 8, out_h: 8, pad_px: 1, screen_scale: 1.0, screen_radius_px: 8.0 * 0.016 };
        let scene = Scene {
            screen: Panel { rect: RectF { x: 2.0, y: 2.0, w: 4.0, h: 4.0 }, radius: 0.0, alpha: 1.0 },
            camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0 },
        };
        let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
        let out = c.composite(&screen, 4, 4, None, cam, &bg, &layout, &scene);
        assert_eq!(out.len(), 8 * 8 * 4);
        assert_eq!(&out[0..4], &[255, 0, 0, 255], "corner must be bg blue");
        let i = ((3 * 8 + 3) * 4) as usize;
        assert_eq!(&out[i..i + 4], &[0, 0, 255, 255], "panel interior must be screen red");
    }

    #[test]
    fn cpu_gpu_parity_two_panels() {
        use crate::export::compositor::{Compositor, CpuCompositor};
        let g = match GpuCompositor::new(64, 48) { Some(c) => c, None => return };
        let screen = solid(32, 24, [10, 20, 200, 255]);   // BGRA-ish
        let webcam = solid(16, 16, [200, 30, 10, 255]);
        let bg = solid(64, 48, [40, 40, 40, 255]);
        let layout = Layout { out_w: 64, out_h: 48, pad_px: 4, screen_scale: 1.0, screen_radius_px: 48.0 * 0.016 };
        let scene = Scene {
            screen: Panel { rect: RectF { x: 8.0, y: 6.0, w: 30.0, h: 22.0 }, radius: 0.0, alpha: 1.0 },
            camera: Panel { rect: RectF { x: 40.0, y: 26.0, w: 18.0, h: 18.0 }, radius: 0.0, alpha: 1.0 },
        };
        let cam = Camera { cx: 32.0, cy: 24.0, scale: 1.0 };
        let cpu = CpuCompositor.composite(&screen, 32, 24, Some((&webcam, 16, 16)), cam, &bg, &layout, &scene);
        let gpu = g.composite(&screen, 32, 24, Some((&webcam, 16, 16)), cam, &bg, &layout, &scene);
        // Interior sample points (centers of bg / screen panel / camera panel) must match within quantization.
        for &(x, y) in &[(2u32, 2u32), (20, 14), (48, 34)] {
            let i = ((y * 64 + x) * 4) as usize;
            for c in 0..4 {
                let d = (cpu[i + c] as i32 - gpu[i + c] as i32).abs();
                assert!(d <= 2, "CPU/GPU mismatch at ({x},{y}) ch {c}: {} vs {}", cpu[i + c], gpu[i + c]);
            }
        }
    }
}
