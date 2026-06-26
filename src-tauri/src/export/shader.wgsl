// Single-pass compositor: bg -> screen (inset, cropped) -> webcam overlay.
// All textures are Bgra8Unorm so bytes stay BGRA end-to-end.

struct Uniforms {
    inset_min: vec2<f32>,   // output-UV rect for the screen inset
    inset_max: vec2<f32>,
    crop_min: vec2<f32>,    // screen-UV rect (camera crop)
    crop_max: vec2<f32>,
    ov_min: vec2<f32>,      // output-UV rect for the webcam overlay
    ov_max: vec2<f32>,
    overlay_enabled: f32,
    is_circle: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var bg_tex: texture_2d<f32>;
@group(0) @binding(1) var screen_tex: texture_2d<f32>;
@group(0) @binding(2) var webcam_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle: 3 vertices, no vertex buffer.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32((vi << 1u) & 2u);   // 0,2,0
    let y = f32(vi & 2u);           // 0,0,2
    out.uv = vec2<f32>(x, y);       // 0..2 in UV space
    out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return out;
}

fn inside(p: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>) -> bool {
    return p.x >= lo.x && p.x < hi.x && p.y >= lo.y && p.y < hi.y;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv;
    var color = textureSample(bg_tex, samp, p);

    if (inside(p, u.inset_min, u.inset_max)) {
        let t = (p - u.inset_min) / (u.inset_max - u.inset_min);
        let screen_uv = u.crop_min + t * (u.crop_max - u.crop_min);
        color = textureSample(screen_tex, samp, screen_uv);
    }

    if (u.overlay_enabled > 0.5 && inside(p, u.ov_min, u.ov_max)) {
        let t = (p - u.ov_min) / (u.ov_max - u.ov_min);
        let local = t * 2.0 - vec2<f32>(1.0, 1.0); // [-1,1]
        if (!(u.is_circle > 0.5 && length(local) > 1.0)) {
            color = textureSample(webcam_tex, samp, t);
        }
    }

    return color;
}
