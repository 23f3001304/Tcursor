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
    bg_uploaded: bool,
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
        let u = build_uniforms(scene, cam, layout, webcam.is_some());

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
        if !r.bg_uploaded {
            g.update_tex(&r.bg_tex, bg, ow, oh);
            r.bg_uploaded = true;
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
        let screen = crate::export::color::bgra_to_nv12(&solid(4, 4, [0, 0, 255, 255]), 4, 4);
        let bg = solid(8, 8, [255, 0, 0, 255]);
        let layout = Layout { out_w: 8, out_h: 8, pad_px: 1, screen_scale: 1.0, screen_radius_px: 8.0 * 0.016 };
        let scene = Scene {
            screen: Panel { rect: RectF { x: 2.0, y: 2.0, w: 4.0, h: 4.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
            camera: Panel { rect: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, radius: 0.0, alpha: 0.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        };
        let cam = Camera { cx: 4.0, cy: 4.0, scale: 1.0 };
        let mut out = Vec::new();
        c.composite_into(&screen, 4, 4, None, cam, &bg, &layout, &scene, &mut out);
        assert_eq!(out.len(), 8 * 8 * 4);
        assert_eq!(&out[0..4], &[255, 0, 0, 255], "corner must be bg blue");
        let i = ((3 * 8 + 3) * 4) as usize;
        let p = &out[i..i + 4]; // screen red through the nv12 shader convert (exact to a few LSBs)
        assert!(p[0] <= 3 && p[1] <= 3 && p[2] >= 250, "panel interior must be ~screen red, got {p:?}");
    }

    #[test]
    fn cpu_gpu_parity_two_panels() {
        use crate::export::gpu::compositor::{Compositor, CpuCompositor};
        let g = match GpuCompositor::new(64, 48) { Some(c) => c, None => return };
        let screen = crate::export::color::bgra_to_nv12(&solid(32, 24, [10, 20, 200, 255]), 32, 24);
        let webcam = solid(16, 16, [200, 30, 10, 255]);
        let bg = solid(64, 48, [40, 40, 40, 255]);
        let layout = Layout { out_w: 64, out_h: 48, pad_px: 4, screen_scale: 1.0, screen_radius_px: 48.0 * 0.016 };
        let scene = Scene {
            screen: Panel { rect: RectF { x: 8.0, y: 6.0, w: 30.0, h: 22.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
            camera: Panel { rect: RectF { x: 40.0, y: 26.0, w: 18.0, h: 18.0 }, radius: 0.0, alpha: 1.0, ring_px: 0.0, ring_color: [0, 0, 0] },
        };
        let cam = Camera { cx: 32.0, cy: 24.0, scale: 1.0 };
        let mut cpu = Vec::new();
        CpuCompositor.composite_into(&screen, 32, 24, Some((&webcam, 16, 16)), cam, &bg, &layout, &scene, &mut cpu);
        let mut gpu = Vec::new();
        g.composite_into(&screen, 32, 24, Some((&webcam, 16, 16)), cam, &bg, &layout, &scene, &mut gpu);
        // Interior sample points (centers of bg / screen panel / camera panel) must match within quantization.
        for &(x, y) in &[(2u32, 2u32), (20, 14), (48, 34)] {
            let i = ((y * 64 + x) * 4) as usize;
            for c in 0..4 {
                let d = (cpu[i + c] as i32 - gpu[i + c] as i32).abs();
                // <=3: the screen panel now goes through two independent nv12->RGB converts (CPU
                // u8-rounded `nv12_to_bgra` vs the shader's float math), so allow one extra LSB.
                assert!(d <= 3, "CPU/GPU mismatch at ({x},{y}) ch {c}: {} vs {}", cpu[i + c], gpu[i + c]);
            }
        }
    }
}
