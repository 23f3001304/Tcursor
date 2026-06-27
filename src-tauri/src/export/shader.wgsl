// Two-panel compositor: zoom the base scene (background + screen panel) toward a
// point, then draw the fixed camera panel on top. Every panel is a rounded rect
// (circle = radius min(w,h)/2). All textures Bgra8Unorm so bytes stay BGRA.

struct Uniforms {
    screen_min: vec2<f32>, screen_max: vec2<f32>,
    cam_min: vec2<f32>, cam_max: vec2<f32>,
    zoom_center: vec2<f32>,
    inv_scale: f32,
    screen_r: f32, camera_r: f32,
    screen_a: f32, camera_a: f32,
    screen_w: f32, screen_h: f32,
    cam_w: f32, cam_h: f32,
    _pad: f32,
};

@group(0) @binding(0) var bg_tex: texture_2d<f32>;
@group(0) @binding(1) var screen_tex: texture_2d<f32>;
@group(0) @binding(2) var webcam_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var<uniform> u: Uniforms;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32((vi << 1u) & 2u);
    let y = f32(vi & 2u);
    out.uv = vec2<f32>(x, y);
    out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return out;
}

fn inside(p: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>) -> bool {
    return p.x >= lo.x && p.x < hi.x && p.y >= lo.y && p.y < hi.y;
}

// Rounded-rect coverage at point `p` (same UV space as lo/hi), feathered inward ~1px.
fn rrect_cov(p: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>, w: f32, h: f32, r: f32) -> f32 {
    let local = (p - lo) / (hi - lo) * vec2<f32>(w, h);
    let half = vec2<f32>(w, h) * 0.5;
    let rr = min(r, min(w, h) * 0.5);
    let q = abs(local - half) - (half - vec2<f32>(rr, rr));
    let d = min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - rr;
    return clamp(0.5 - d, 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv;
    let base_uv = u.zoom_center + (p - vec2<f32>(0.5, 0.5)) * u.inv_scale;
    var color = textureSample(bg_tex, samp, base_uv);
    // Screen panel: in the zoomed base scene.
    if (u.screen_a > 0.001 && inside(base_uv, u.screen_min, u.screen_max)) {
        let suv = (base_uv - u.screen_min) / (u.screen_max - u.screen_min);
        let cov = rrect_cov(base_uv, u.screen_min, u.screen_max, u.screen_w, u.screen_h, u.screen_r) * u.screen_a;
        color = mix(color, textureSample(screen_tex, samp, suv), cov);
    }
    // Camera panel: fixed in OUTPUT space, on top (not zoomed).
    if (u.camera_a > 0.001 && inside(p, u.cam_min, u.cam_max)) {
        let cuv = (p - u.cam_min) / (u.cam_max - u.cam_min);
        let cov = rrect_cov(p, u.cam_min, u.cam_max, u.cam_w, u.cam_h, u.camera_r) * u.camera_a;
        color = mix(color, textureSample(webcam_tex, samp, cuv), cov);
    }
    return color;
}
