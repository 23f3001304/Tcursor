use wgpu::util::DeviceExt;
use crate::export::compositor::Compositor;
use crate::export::gpu::Gpu;
use crate::export::types::{Camera, Layout, OverlayLayout, OverlayPos, OverlayShape};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    inset_min: [f32; 2],
    inset_max: [f32; 2],
    crop_min: [f32; 2],
    crop_max: [f32; 2],
    ov_min: [f32; 2],
    ov_max: [f32; 2],
    overlay_enabled: f32,
    is_circle: f32,
    _pad: [f32; 2],
}

pub struct GpuCompositor {
    gpu: Gpu,
    out_w: u32,
    out_h: u32,
}

impl GpuCompositor {
    pub fn new(out_w: u32, out_h: u32) -> Option<GpuCompositor> {
        Some(GpuCompositor { gpu: Gpu::new(out_w, out_h)?, out_w, out_h })
    }
}

/// Build the uniform: inset rect (output-UV), crop rect (screen-UV), overlay rect.
fn build_uniforms(
    sw: u32, sh: u32, cam: Camera, layout: &Layout,
    overlay: &OverlayLayout, has_webcam: bool,
) -> Uniforms {
    let (ow, oh) = (layout.out_w as f32, layout.out_h as f32);
    let pad = layout.pad_px as f32;
    // Inset rect in output UV (mirror of CpuCompositor's pad blit).
    let inset_min = [pad / ow, pad / oh];
    let inset_max = [(ow - pad) / ow, (oh - pad) / oh];

    // Crop rect in screen UV (mirror of CpuCompositor's resize_crop).
    let cw = (sw as f32 / cam.scale).round();
    let ch = (sh as f32 / cam.scale).round();
    let cx0 = (cam.cx - cw / 2.0).clamp(0.0, (sw as f32 - cw).max(0.0));
    let cy0 = (cam.cy - ch / 2.0).clamp(0.0, (sh as f32 - ch).max(0.0));
    let crop_min = [cx0 / sw as f32, cy0 / sh as f32];
    let crop_max = [(cx0 + cw) / sw as f32, (cy0 + ch) / sh as f32];

    // Overlay rect in output UV (mirror of overlay_origin + size).
    let sz = overlay.size_px as f32;
    let m = overlay.margin_px as f32;
    let (ox, oy) = match overlay.pos {
        OverlayPos::BottomLeft => (m, (oh - sz - m).max(0.0)),
        OverlayPos::BottomRight => ((ow - sz - m).max(0.0), (oh - sz - m).max(0.0)),
        OverlayPos::TopLeft => (m, m),
        OverlayPos::TopRight => ((ow - sz - m).max(0.0), m),
        OverlayPos::Custom { x, y } => (x as f32, y as f32),
    };
    let enabled = overlay.enabled && has_webcam;
    let is_circle = matches!(overlay.shape, OverlayShape::Circle);

    Uniforms {
        inset_min, inset_max, crop_min, crop_max,
        ov_min: [ox / ow, oy / oh],
        ov_max: [(ox + sz) / ow, (oy + sz) / oh],
        overlay_enabled: if enabled { 1.0 } else { 0.0 },
        is_circle: if is_circle { 1.0 } else { 0.0 },
        _pad: [0.0, 0.0],
    }
}

impl Compositor for GpuCompositor {
    fn composite(
        &self,
        screen: &[u8], sw: u32, sh: u32,
        webcam: Option<(&[u8], u32, u32)>,
        cam: Camera, bg: &[u8],
        layout: &Layout,
        overlay: &OverlayLayout,
    ) -> Vec<u8> {
        let g = &self.gpu;
        let (ow, oh) = (self.out_w, self.out_h);

        let screen_tex = g.upload_tex("screen", screen, sw, sh);
        let bg_tex = g.upload_tex("bg", bg, ow, oh);
        let (wc_data, ww, wh) = webcam.unwrap_or((&[0u8; 4], 1, 1));
        let wc_tex = g.upload_tex("webcam", wc_data, ww, wh);

        let sv = screen_tex.create_view(&Default::default());
        let bv = bg_tex.create_view(&Default::default());
        let wv = wc_tex.create_view(&Default::default());

        let u = build_uniforms(sw, sh, cam, layout, overlay, webcam.is_some());
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
    use crate::export::types::{Camera, Layout, OverlayLayout};

    fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        let mut v = vec![0u8; (w * h * 4) as usize];
        for c in v.chunks_mut(4) { c.copy_from_slice(&px); }
        v
    }

    #[test]
    fn composites_onto_background_at_inset() {
        // Mirror of CpuCompositor::composites_onto_background_at_inset; skip if no GPU.
        let c = match GpuCompositor::new(8, 8) { Some(c) => c, None => return };
        let sw = 4u32; let sh = 4u32;
        let screen = solid(sw, sh, [0, 0, 255, 255]); // red (BGRA)
        let bg = solid(8, 8, [255, 0, 0, 255]);        // blue (BGRA)
        let layout = Layout { out_w: 8, out_h: 8, pad_px: 1 };
        let overlay = OverlayLayout { enabled: false, ..OverlayLayout::default() };
        let cam = Camera { cx: 2.0, cy: 2.0, scale: 1.0 };
        let out = c.composite(&screen, sw, sh, None, cam, &bg, &layout, &overlay);
        assert_eq!(out.len(), 8 * 8 * 4);
        assert_eq!(&out[0..4], &[255, 0, 0, 255], "corner must be bg blue");
        let i = ((3 * 8 + 3) * 4) as usize;
        assert_eq!(&out[i..i + 4], &[0, 0, 255, 255], "inset (3,3) must be screen red");
    }
}
