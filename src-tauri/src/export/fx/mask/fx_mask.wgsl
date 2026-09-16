// Masks: blur, pixelate, highlight. Concatenated onto fx.wgsl by fx_gpu_pipeline.rs, so `u`,
// `frame_tex` and `samp` are fx.wgsl's. Slot i is u.mask[3i..3i+2]; kind id 0 means empty.
// Ids must mirror fx_masks::mask_kind_id exactly.
const MK_BLUR: f32 = 1.0;
const MK_PIXELATE: f32 = 2.0;
const MK_HIGHLIGHT: f32 = 3.0;

fn rrect_sd(p: vec2<f32>, mn: vec2<f32>, mx: vec2<f32>, r: f32) -> f32 {
  let c = (mn + mx) * 0.5;
  let hb = (mx - mn) * 0.5 - vec2<f32>(r, r);
  let q = abs(p - c) - hb;
  return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn mask_cov(p: vec2<f32>, i: i32) -> f32 {
  let a = u.mask[i * 3];
  let b = u.mask[i * 3 + 1];
  let sd = rrect_sd(p, a.xy, a.zw, b.x);
  return clamp(0.5 - sd / max(b.y, 1.0), 0.0, 1.0) * u.mask[i * 3 + 2].y;
}

// A fixed 13 tap disc in one pass: the cost does not grow with the radius, only the spread does.
// The CPU path runs three box passes instead, so a soft gradient can differ in its last bit.
fn mask_blur(uv: vec2<f32>, dims: vec2<f32>, r: f32) -> vec3<f32> {
  var disc = array<vec2<f32>, 13>(
    vec2<f32>( 0.0,  0.0), vec2<f32>( 1.0,  0.0), vec2<f32>(-1.0,  0.0),
    vec2<f32>( 0.0,  1.0), vec2<f32>( 0.0, -1.0), vec2<f32>( 0.7,  0.7),
    vec2<f32>(-0.7,  0.7), vec2<f32>( 0.7, -0.7), vec2<f32>(-0.7, -0.7),
    vec2<f32>( 0.5,  0.0), vec2<f32>(-0.5,  0.0), vec2<f32>( 0.0,  0.5),
    vec2<f32>( 0.0, -0.5));
  var acc = vec3<f32>(0.0, 0.0, 0.0);
  for (var k = 0; k < 13; k = k + 1) {
    acc = acc + textureSample(frame_tex, samp, uv + disc[k] * r / dims).rgb;
  }
  return acc / 13.0;
}

fn mask_fx(col: vec3<f32>, px: vec2<f32>, dims: vec2<f32>, uv: vec2<f32>) -> vec3<f32> {
  var c = col;
  for (var i = 0; i < 8; i = i + 1) {
    let a = u.mask[i * 3];
    let b = u.mask[i * 3 + 1];
    let m = u.mask[i * 3 + 2];
    // The branch is on a uniform, which keeps the implicit-derivative analysis happy.
    if (b.w >= MK_BLUR - 0.5) {
      let cov = mask_cov(px, i);
      if (b.w > MK_HIGHLIGHT - 0.5) {
        c = c * (1.0 - m.x * m.y * (1.0 - cov));
      } else if (b.w < MK_PIXELATE - 0.5) {
        c = mix(c, mask_blur(uv, dims, max(b.z, 1.0)), cov);
      } else {
        let cell = max(2.0, b.z);
        let q = floor((px - a.xy) / cell) * cell + a.xy + cell * 0.5;
        c = mix(c, textureSample(frame_tex, samp, q / dims).rgb, cov);
      }
    }
  }
  return c;
}
