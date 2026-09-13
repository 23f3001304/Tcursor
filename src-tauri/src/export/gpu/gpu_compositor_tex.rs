// Texture + bind-group construction for `GpuCompositor::composite_into`, split out
// so gpu_compositor.rs stays under the size limit. Pure builder: identical to the
// inline block it replaces, no behavior change.
use wgpu::util::DeviceExt;
use crate::export::gpu::gpu_uniforms::Uniforms;
use crate::export::gpu::Gpu;
use super::CompositorResources;

/// A cheap change key for the background buffer: its length plus an FNV-1a over ~4k evenly
/// strided 4-byte samples. `FrameRenderer::reload_edit` rebuilds `bg` at the SAME dimensions,
/// so the resource-rebuild check (`sw/sh/ww/wh`) can never see a background edit and the warm
/// preview kept sampling a stale `bg_tex`; hashing all ~8 MB every frame would cost more than
/// the upload it saves, and a background (fill, gradient, blur, wallpaper) that changes at all
/// changes it across the whole frame, so a strided sample sees it.
pub(super) fn bg_key(bg: &[u8]) -> u64 {
    const FNV: u64 = 0x100000001b3;
    let mut h = (0xcbf29ce484222325u64 ^ bg.len() as u64).wrapping_mul(FNV);
    let words = bg.len() / 4;
    for i in (0..words).step_by((words / 4096).max(1)) {
        let w = u32::from_le_bytes([bg[i * 4], bg[i * 4 + 1], bg[i * 4 + 2], bg[i * 4 + 3]]);
        h = (h ^ w as u64).wrapping_mul(FNV);
    }
    h
}

/// Does this frame's background need uploading to the GPU?
///
/// A STATIC background uploads only when its content key moved - that is the whole point of
/// `bg_key`, and it saves an ~8 MB texture write on every frame of a normal export. A DYNAMIC one
/// (a video/GIF asset, `background::video_source`) uploads unconditionally, because the key is a
/// ~4096-pixel SAMPLE: a video frame that moves only a small region can hash equal to the frame
/// before it, and the difference between "probably changed" and "changed" is a background frozen
/// on screen. The caller skips computing the key at all in that case, so this is also cheaper.
pub(super) fn should_upload(key_changed: bool, dynamic: bool) -> bool { key_changed || dynamic }

/// Build the screen/webcam/bg textures, the uniform buffer, and the bind group for one
/// `(sw, sh, ww, wh)` size combination. Called only when `composite_into` detects a
/// size change (or on the first frame) - `bg_key` always starts `None` so the
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
    CompositorResources { sw, sh, ww, wh, screen_y_tex, screen_uv_tex, webcam_tex, bg_tex, ubuf, bind, bg_key: None }
}

#[cfg(test)]
mod tests {
    use super::{bg_key, should_upload};

    /// The upload rule itself. A video background must re-upload even when the sampled key
    /// repeats; a static one must still upload only on a real change.
    #[test]
    fn a_dynamic_background_always_uploads_a_static_one_only_on_change() {
        assert!(should_upload(false, true));
        assert!(should_upload(true, true));
        assert!(should_upload(true, false));
        assert!(!should_upload(false, false));
    }

    /// The key must move when the background's PIXELS move (an edit rebuilds `bg` at the same
    /// dimensions, so length alone can never tell) and must be stable for an identical buffer.
    #[test]
    fn bg_key_tracks_content_not_just_length() {
        let a = vec![7u8; 1920 * 1080 * 4];
        assert_eq!(bg_key(&a), bg_key(&vec![7u8; 1920 * 1080 * 4]));
        let mut b = a.clone();
        for px in b.chunks_mut(4) { px[1] = 9; } // a colour/gradient/blur change touches every pixel
        assert_ne!(bg_key(&a), bg_key(&b));
        assert_ne!(bg_key(&a), bg_key(&a[..a.len() - 4])); // different size, different key
        assert_eq!(bg_key(&[]), bg_key(&[])); // degenerate: no panic, stable
    }
}
