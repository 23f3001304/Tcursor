// FX pass: sample the composited frame, dim outside the spotlight, then hand it to `clicks()` in
// the sibling fx_clicks.wgsl (concatenated onto this file by fx_gpu.rs) for the click styles.
// Logical-RGBA; Bgra8Unorm handles byte order.
// Style ids - must mirror fx_uniforms::style_id() exactly.
const FX_RIPPLE: f32 = 1.0;
const FX_PULSE: f32 = 2.0;
const FX_GLOW: f32 = 3.0;
const FX_SHOCKWAVE: f32 = 4.0;
const FX_PARTICLES: f32 = 5.0;
const FX_NEON: f32 = 6.0;

// Spotlight mode ids - must mirror fx_uniforms::spot_mode_id() exactly.
const SP_CLASSIC: f32 = 0.0;
const SP_BLUR: f32 = 1.0;
const SP_HALO: f32 = 2.0;
const SP_BREATHING: f32 = 3.0;
const SP_NEBULA: f32 = 4.0;
const SP_VIGNETTE: f32 = 5.0;

// Video FX mode ids - must mirror fx_uniforms::video_mode_id() exactly.
const VF_NEBULA: f32 = 0.0;
const VF_CINEMATIC: f32 = 1.0;
const VF_FOCUS: f32 = 2.0;
const VF_COLORPOP: f32 = 3.0;

struct FxU {
  a: vec4<f32>,                 // ow, oh, style, hit_count
  b: vec4<f32>,                 // spot_cx, spot_cy, spot_dim_alpha, spot_active
  c: vec4<f32>,                 // spot_r_in, spot_r_out, intensity, _pad
  d: vec4<f32>,                 // spot_mode_id, time_s, keep_camera_lit(0/1), cam_radius(px)
  tint: vec4<f32>,              // r, g, b 0..1, _pad
  color: vec4<f32>,             // rgb 0..1, _pad
  color2: vec4<f32>,            // rgb 0..1 (the tint rotated 30 deg of hue, Neon's 2nd tube), _pad
  hits: array<vec4<f32>, 16>,   // x, y, progress, _pad
  e: vec4<f32>,                 // video_mode_id, alpha, t, _pad
  cam: vec4<f32>,               // camera-exclusion rect (px): min_x, min_y, max_x, max_y
  // The glass cursor material (fx_lens.wgsl). Both shapes are off when their `on` slot is 0.
  lens_a: vec4<f32>,            // sprite lens box (px): centre x, centre y, w, h
  lens_b: vec4<f32>,            // busy angle (rad), on(0/1), click squash, ink progress (<0 = none)
  lens_c: vec4<f32>,            // ink origin (px): x, y, _pad, _pad
  back_a: vec4<f32>,            // cursor-back rounded rect (px): min_x, min_y, max_x, max_y
  back_b: vec4<f32>,            // corner radius(px), on(0/1), click squash, ink progress
  back_c: vec4<f32>,            // ink origin (px): x, y, ring-instead-of-drop(0/1), _pad
  mask: array<vec4<f32>, 16>,   // reserved for Batch 2a: two vec4 per mask, eight masks, all zero until then
  grade: array<vec4<f32>, 6>,   // reserved for Batch 2b: the grade parameters, all zero until then
};
@group(0) @binding(0) var frame_tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> u: FxU;

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

fn hash2(p: vec2<f32>) -> f32{ return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise2(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p); let u2 = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), u2.x),
             mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u2.x), u2.y);
}
fn fbm(p: vec2<f32>) -> f32 {
  var v = 0.0; var a = 0.5; var q = p;
  for (var i = 0; i < 4; i = i + 1) { v = v + a * noise2(q); q = q * 2.0; a = a * 0.5; }
  return v;
}

// Domain-warped, multi-color nebula (deep indigo -> violet -> magenta -> blue).
// Returns a color in 0..1; callers blend it in (do not add) to avoid blow-out.
fn nebula(p: vec2<f32>, tm: f32) -> vec3<f32> {
  let q = vec2<f32>(fbm(p + tm * 0.04), fbm(p + vec2<f32>(5.2, 1.3) + tm * 0.035));
  let r = vec2<f32>(fbm(p + 3.5 * q + vec2<f32>(1.7, 9.2) + tm * 0.03), fbm(p + 3.5 * q + vec2<f32>(8.3, 2.8)));
  let f = fbm(p + 3.5 * r);
  let deep = vec3<f32>(0.03, 0.01, 0.09);
  let viol = vec3<f32>(0.24, 0.09, 0.42);
  let hot  = vec3<f32>(0.60, 0.20, 0.52);
  let blu  = vec3<f32>(0.10, 0.28, 0.58);
  var c = mix(deep, viol, clamp(f * 1.5, 0.0, 1.0));
  c = mix(c, blu, clamp(dot(q, q) * 0.9, 0.0, 1.0) * 0.45);
  c = mix(c, hot, clamp(length(r) * 0.8, 0.0, 1.0) * 0.8);
  return c;
}

fn rrect_cov(p: vec2<f32>, mn: vec2<f32>, mx: vec2<f32>, r: f32) -> f32 { // ~1 inside, ~0 outside
  let q = abs(p - (mn + mx) * 0.5) - ((mx - mn) * 0.5 - vec2<f32>(r, r));
  let sd = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
  return clamp(0.5 - sd, 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let dims = vec2<f32>(u.a.x, u.a.y);
  let oh = u.a.y;
  let style = u.a.z;
  let n = i32(u.a.w);
  var uv = in.uv;
  let px = in.uv * dims;
  // Shockwave warps the sampled UV radially near each expanding ring. The band formula is
  // unchanged; only its radius now eases like every other click radius (`fx_ease`), because the
  // ring fx_clicks.wgsl draws sits ON this band - let one ease and not the other and the glass
  // separates from its own highlight.
  var disp = vec2<f32>(0.0, 0.0);
  if (style == FX_SHOCKWAVE) {
    for (var i = 0; i < n; i = i + 1) {
      let h = u.hits[i];
      let dir = px - h.xy;
      let d = length(dir);
      let radius = fx_ease(clamp(h.z, 0.0, 1.0)) * oh * 0.09;
      let band = clamp(1.0 - abs(d - radius) / max(oh * 0.03, 1.0), 0.0, 1.0);
      let amp = oh * 0.02 * band * (1.0 - clamp(h.z, 0.0, 1.0));
      disp = disp + normalize(dir + vec2<f32>(0.0001, 0.0)) * amp;
    }
    uv = uv - disp / dims;
  }
  var color = textureSample(frame_tex, samp, uv).rgb;
  // Chromatic dispersion across the band: R and B come from 2 px either side of the displaced
  // point ALONG the displacement, G from the displaced point itself. Away from the band `disp`
  // is zero, so the offset is zero and all three channels are the plain sample. The branch is on
  // a uniform (`style`), which keeps the extra samples off every other style AND keeps the
  // implicit-derivative uniformity analysis happy.
  if (style == FX_SHOCKWAVE) {
    let ofs = disp / max(length(disp), 0.0001) * 2.0 / dims;
    color = vec3<f32>(textureSample(frame_tex, samp, uv + ofs).r, color.g,
                      textureSample(frame_tex, samp, uv - ofs).b);
  }
  if (u.e.y > 0.001) {
    let vmode = u.e.x;
    let va = u.e.y;
    let vtime = u.e.z;
    if (vmode == VF_NEBULA) {
      let neb = nebula(px / oh * 2.6, vtime);
      let cl = 0.35 + 0.65 * fbm(px / oh * 2.0 + vec2<f32>(vtime * 0.05, 0.0));
      color = mix(color, neb, va * 0.40 * cl);
    } else if (vmode == VF_CINEMATIC) {
      let vg = clamp(distance(px, dims * 0.5) / length(dims * 0.5), 0.0, 1.0);
      color = color * (1.0 - va * (0.2 + 0.5 * vg));
    } else if (vmode == VF_FOCUS) {
      let m = dims * 0.08;
      let inside = px.x > m.x && px.y > m.y && px.x < dims.x - m.x && px.y < dims.y - m.y;
      if (!inside) { color = color * (1.0 - va * 0.6); }
    } else if (vmode == VF_COLORPOP) {
      let lum = dot(color, vec3<f32>(0.299, 0.587, 0.114));
      color = mix(color, clamp((color - lum) * 1.6 + lum, vec3<f32>(0.0), vec3<f32>(1.0)), va);
    }
  }
  let pre_spot = color;
  if (u.b.w > 0.5) {
    let mode = u.d.x;
    let time = u.d.y;
    if (mode == SP_VIGNETTE) {
      let half = dims * 0.5;
      let vt = clamp((distance(px, half) / (length(half)) - 0.4) / 0.6, 0.0, 1.0);
      color = color * (1.0 - u.b.z * vt);
    } else {
      var ri = u.c.x;
      var ro = u.c.y;
      if (mode == SP_BREATHING) { let bb = 1.0 + 0.12 * sin(time * 3.1416); ri = ri * bb; ro = ro * bb; }
      let dd = distance(px, u.b.xy);
      let t = clamp((dd - ri) / max(ro - ri, 0.001), 0.0, 1.0);
      if (mode == SP_BLUR) {
        let o = oh * 0.004;
        var bl = textureSample(frame_tex, samp, in.uv + vec2<f32>(o, 0.0) / dims).rgb
               + textureSample(frame_tex, samp, in.uv - vec2<f32>(o, 0.0) / dims).rgb
               + textureSample(frame_tex, samp, in.uv + vec2<f32>(0.0, o) / dims).rgb
               + textureSample(frame_tex, samp, in.uv - vec2<f32>(0.0, o) / dims).rgb;
        color = mix(color, bl * 0.25 * (1.0 - u.b.z * 0.5), t);
      } else if (mode == SP_NEBULA) {
        let neb = nebula(px / oh * 2.6, time);
        let inten = clamp(u.c.z, 0.0, 1.0);
        color = color * (1.0 - 0.80 * t);
        color = mix(color, neb, (1.0 - t) * 0.48 * inten);
        color = color + vec3<f32>(0.45, 0.26, 0.78) * exp(-(dd * dd) / max(ri * ri, 1.0)) * inten * 0.15;
      } else {
        color = color * (1.0 - u.b.z * t);
        if (mode == SP_HALO) {
          let band = clamp(1.0 - abs(dd - ri) / max(oh * 0.02, 1.0), 0.0, 1.0);
          color = color + u.tint.rgb * band * u.c.z;
        }
      }
    }
  }
  color = mix(color, pre_spot, u.d.z * rrect_cov(px, u.cam.xy, u.cam.zw, u.d.w));
  color = clicks(color, px, oh, style, n); // fx_clicks.wgsl - every click style lives there
  // Last, so the glass sits over the spotlight and the click styles, and the sprite blit that
  // follows this whole pass lands on top of it. What it REFRACTS is `frame_tex` (it has to
  // re-sample, and only the uploaded frame can be re-sampled) - same trade Shockwave makes above.
  // The spotlight is centred on the cursor, so the undimmed sample is a fraction of a percent off.
  color = lens_fx(color, px);              // fx_lens.wgsl - the glass cursor material
  return vec4<f32>(min(color, vec3<f32>(1.0, 1.0, 1.0)), 1.0);
}
