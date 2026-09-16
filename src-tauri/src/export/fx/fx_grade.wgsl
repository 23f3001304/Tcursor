// The colour grade. Concatenated onto fx.wgsl by fx_gpu_pipeline.rs, so `u` is fx.wgsl's.
// A line for line transcription of export::grade::apply_px; the order is fixed and changing it
// changes every existing graded project. sRGB in, sRGB out, no linearisation.
// grade[0] exposure, contrast, saturation, vignette
// grade[1] temp, tint, active(0/1), _pad
// grade[2] lift rgb   grade[3] gamma rgb   grade[4] gain rgb   grade[5] spare
const G_LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);
const G_VIGN_IN: f32 = 0.45;
const G_CORNER: f32 = 0.70710678;
const G_TEMP: f32 = 0.25;
const G_TINT: f32 = 0.20;

fn grade_fx(col: vec3<f32>, px: vec2<f32>, dims: vec2<f32>) -> vec3<f32> {
  let a = u.grade[0];
  let b = u.grade[1];
  if (b.z < 0.5) { return col; }
  let lift = u.grade[2].rgb;
  let gam = u.grade[3].rgb;
  let gain = u.grade[4].rgb;
  let e = exp2(a.x);
  var c = vec3<f32>(col.r * e * (1.0 + G_TEMP * b.x),
                    col.g * e * (1.0 + G_TINT * b.y),
                    col.b * e * (1.0 - G_TEMP * b.x));
  c = clamp(lift + (gain - lift) * c, vec3<f32>(0.0), vec3<f32>(1.0));
  c = pow(c, vec3<f32>(1.0) / max(gam, vec3<f32>(0.001)));
  c = (c - vec3<f32>(0.5)) * a.y + vec3<f32>(0.5);
  let l = dot(c, G_LUMA);
  c = vec3<f32>(l) + (c - vec3<f32>(l)) * a.z;
  let uv = px / dims - vec2<f32>(0.5);
  let d = length(uv) / G_CORNER;
  let k = clamp((d - G_VIGN_IN) / (1.0 - G_VIGN_IN), 0.0, 1.0);
  return clamp(c * (1.0 - a.w * k * k), vec3<f32>(0.0), vec3<f32>(1.0));
}
