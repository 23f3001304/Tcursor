// The webcam panel's ring/border band, split out of `compositor.rs` so that file stays under the
// line budget. Pure pixel work, no behavior change: this is the same loop `draw_panel` called
// inline, moved verbatim.
use super::rrect_sd_px;

/// Blend `ring_color` over `dst` in a band just inside the panel edge, width `ring_px`,
/// weighted by panel alpha `a`. Same SDF/band math as the WGSL shader's ring blend. `ox`/`oy`
/// are SIGNED (see `draw_panel`) - an off-edge panel clips instead of snapping to the corner.
pub(super) fn blit_ring(dst: &mut [u8], dw: u32, dh: u32, pw: u32, ph: u32, ox: i32, oy: i32,
                        r: f32, ring_px: f32, ring_color: [u8; 3], a: f32) {
    let [rr, rg, rb] = ring_color;
    for ty in 0..ph {
        for tx in 0..pw {
            let d = rrect_sd_px(tx, ty, pw, ph, r);
            if d > 0.0 || d < -ring_px { continue; } // outside the panel or inside the ring band
            let band = ((ring_px + d) / ring_px.max(1.0)).clamp(0.0, 1.0) * a;
            if band <= 0.0 { continue; }
            let (dx, dy) = (ox + tx as i32, oy + ty as i32);
            if dx < 0 || dy < 0 || dx as u32 >= dw || dy as u32 >= dh { continue; }
            let (dx, dy) = (dx as u32, dy as u32);
            let di = ((dy * dw + dx) * 4) as usize;
            // BGRA destination bytes; ring_color is RGB.
            dst[di] = (rb as f32 * band + dst[di] as f32 * (1.0 - band)).round() as u8;
            dst[di + 1] = (rg as f32 * band + dst[di + 1] as f32 * (1.0 - band)).round() as u8;
            dst[di + 2] = (rr as f32 * band + dst[di + 2] as f32 * (1.0 - band)).round() as u8;
        }
    }
}
