// FX pass: sample the composited frame, dim outside the spotlight, draw click
// rings (ripple) / discs (pulse). Logical-RGBA; Bgra8Unorm handles byte order.
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
  d: vec4<f32>,                 // spot_mode_id, time_s, _pad, _pad
  tint: vec4<f32>,              // r, g, b 0..1, _pad
  color: vec4<f32>,             // rgb 0..1, _pad
  hits: array<vec4<f32>, 16>,   // x, y, progress, _pad
  e: vec4<f32>,                 // video_mode_id, alpha, t, _pad
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

fn hash1(x: f32) -> f32 { return fract(sin(x * 127.1) * 43758.5453); }
fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
  let dims = vec2<f32>(u.a.x, u.a.y);
  let oh = u.a.y;
  let style = u.a.z;
  let n = i32(u.a.w);
  var uv = in.uv;
  let px = in.uv * dims;
  // Shockwave warps the sampled UV radially near each expanding ring.
  if (style == FX_SHOCKWAVE) {
    var disp = vec2<f32>(0.0, 0.0);
    for (var i = 0; i < n; i = i + 1) {
      let h = u.hits[i];
      let dir = px - h.xy;
      let d = length(dir);
      let radius = clamp(h.z, 0.0, 1.0) * oh * 0.09;
      let band = clamp(1.0 - abs(d - radius) / max(oh * 0.03, 1.0), 0.0, 1.0);
      let amp = oh * 0.02 * band * (1.0 - clamp(h.z, 0.0, 1.0));
      disp = disp + normalize(dir + vec2<f32>(0.0001, 0.0)) * amp;
    }
    uv = uv - disp / dims;
  }
  var color = textureSample(frame_tex, samp, uv).rgb;
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
  for (var i = 0; i < n; i = i + 1) {
    let h = u.hits[i];
    let a = clamp(1.0 - h.z, 0.0, 1.0) * clamp(u.c.z, 0.0, 1.0);
    let d = distance(px, h.xy);
    if (style == FX_RIPPLE) {
      let radius = clamp(h.z, 0.0, 1.0) * oh * 0.06;
      let thick = max(oh * 0.006, 1.0);
      color = mix(color, u.color.rgb, a * clamp((thick - abs(d - radius)) / thick, 0.0, 1.0));
    } else if (style == FX_PULSE) {
      color = mix(color, u.color.rgb, a * clamp(oh * 0.02 - d, 0.0, 1.0));
    } else if (style == FX_GLOW) {
      let r = oh * 0.05 * (0.6 + 0.8 * clamp(h.z, 0.0, 1.0));
      color = color + u.color.rgb * exp(-(d * d) / (r * r)) * a;
    } else if (style == FX_NEON) {
      let radius = clamp(h.z, 0.0, 1.0) * oh * 0.07;
      let thick = max(oh * 0.01, 1.0);
      let cov = clamp((thick - abs(d - radius)) / thick, 0.0, 1.0);
      color = color + u.color.rgb * cov * a * 1.4 + vec3<f32>(1.0, 1.0, 1.0) * cov * a * 0.25;
    } else if (style == FX_SHOCKWAVE) {
      let radius = clamp(h.z, 0.0, 1.0) * oh * 0.09;
      let thick = max(oh * 0.008, 1.0);
      color = color + u.color.rgb * clamp((thick - abs(d - radius)) / thick, 0.0, 1.0) * a * 0.5;
    } else if (style == FX_PARTICLES) {
      for (var k = 0; k < 12; k = k + 1) {
        let ang = hash1(f32(k)) * 6.2831853;
        let sp = (0.4 + hash1(f32(k) + 7.0)) * oh * 0.10;
        let prog = clamp(h.z, 0.0, 1.0);
        let pos = h.xy + vec2<f32>(cos(ang), sin(ang)) * sp * prog + vec2<f32>(0.0, oh * 0.06 * prog * prog);
        color = color + u.color.rgb * clamp(oh * 0.004 - distance(px, pos), 0.0, 1.0) * a;
      }
    }
  }
  return vec4<f32>(min(color, vec3<f32>(1.0, 1.0, 1.0)), 1.0);
}
