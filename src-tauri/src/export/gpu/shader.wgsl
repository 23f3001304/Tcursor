// Two-panel compositor: zoom the base scene (background + screen panel) toward a
// point, then draw the fixed camera panel on top. Every panel is a rounded rect
// (circle = radius min(w,h)/2). bg/webcam/output are Bgra8Unorm; the screen arrives
// as nv12 (Y in an R8 texture + interleaved half-res UV in an Rg8 texture) and is
// converted to RGB in `screen_rgb` (BT.601 limited-range, see export::color).

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
    ring: vec4<f32>, // x = ring width (px), yzw = ring color 0..1; x == 0 -> no ring
};

@group(0) @binding(0) var bg_tex: texture_2d<f32>;
@group(0) @binding(1) var screen_y: texture_2d<f32>;
@group(0) @binding(2) var screen_uv: texture_2d<f32>;
@group(0) @binding(3) var webcam_tex: texture_2d<f32>;
@group(0) @binding(4) var samp: sampler;
@group(0) @binding(5) var<uniform> u: Uniforms;

// nv12 screen sample -> RGB. BT.601 limited-range, matching ffmpeg's default yuv420p->bgra so the
// GPU export is byte-identical (within rounding) to the old bgra-decode path. Constants MUST match
// export::color::yuv_to_rgb.
fn screen_rgb(suv: vec2<f32>) -> vec4<f32> {
    let yv = textureSample(screen_y, samp, suv).r * 255.0;
    let c = textureSample(screen_uv, samp, suv).rg * 255.0;
    let l = 1.16438 * (yv - 16.0);
    let cb = c.x - 128.0;
    let cr = c.y - 128.0;
    let r = (l + 1.59603 * cr) / 255.0;
    let g = (l - 0.39176 * cb - 0.81297 * cr) / 255.0;
    let b = (l + 2.01723 * cb) / 255.0;
    return vec4<f32>(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), 1.0);
}

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

// Rounded-rect signed distance (px, same scale as w/h) at point `p` (same UV space
// as lo/hi): negative inside, 0 at the edge, positive outside.
fn rrect_sd(p: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>, w: f32, h: f32, r: f32) -> f32 {
    let local = (p - lo) / (hi - lo) * vec2<f32>(w, h);
    let half = vec2<f32>(w, h) * 0.5;
    let rr = min(r, min(w, h) * 0.5);
    let q = abs(local - half) - (half - vec2<f32>(rr, rr));
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - rr;
}

// Rounded-rect coverage at point `p`, feathered inward ~1px.
fn rrect_cov(p: vec2<f32>, lo: vec2<f32>, hi: vec2<f32>, w: f32, h: f32, r: f32) -> f32 {
    return clamp(0.5 - rrect_sd(p, lo, hi, w, h, r), 0.0, 1.0);
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
        color = mix(color, screen_rgb(suv), cov);
    }
    // Camera panel: fixed in OUTPUT space, on top (not zoomed).
    if (u.camera_a > 0.001 && inside(p, u.cam_min, u.cam_max)) {
        let cuv = (p - u.cam_min) / (u.cam_max - u.cam_min);
        let cov = rrect_cov(p, u.cam_min, u.cam_max, u.cam_w, u.cam_h, u.camera_r) * u.camera_a;
        color = mix(color, textureSample(webcam_tex, samp, cuv), cov);
        // Optional ring/border just inside the camera panel edge.
        if (u.ring.x > 0.0) {
            let d = rrect_sd(p, u.cam_min, u.cam_max, u.cam_w, u.cam_h, u.camera_r);
            let band = clamp((u.ring.x + d) / max(u.ring.x, 1.0), 0.0, 1.0) * step(d, 0.0) * step(-u.ring.x, d);
            color = mix(color, vec4<f32>(u.ring.yzw, 1.0), band * u.camera_a);
        }
    }
    return color;
}
