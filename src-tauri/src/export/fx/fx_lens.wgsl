// The glass cursor material for fx.wgsl. `fx_gpu.rs::build_pipeline` concatenates this file after
// fx.wgsl and fx_clicks.wgsl into ONE module, so `u`, `frame_tex`, `samp` and `fx_ease` all come
// from there; only the lens lives here. `fx_lensdraw.rs` is the CPU stand-in (the magnification
// and the shadow, no rim bend or frost - see its doc), `cursorGlass.ts` the live-canvas
// approximation. When the three disagree, THIS file is right.
//
// The sprite is the LENS, not the finished look: the mask below is its alpha, the colour inside it
// is the frame re-sampled as a MAGNIFIER - everything inside is `LENS_ZOOM` bigger and stays
// readable, and the rim bends a little more on top. The pack's own pixels are blitted at
// `fx_lens::SPRITE_ALPHA` afterwards, by the cursor pass, so its baked highlights and rim sit on
// top of live refraction.
@group(0) @binding(3) var lens_mask: texture_2d<f32>;

// The magnification inside a shape - `fx_lens::ZOOM`, mirrored by `cursorGlass.ts::LENS_ZOOM`.
// Uniform, so the text under the glass is simply bigger, not warped (owner ruling 2026-09-14: the
// glass must zoom AND stay readable - the old rim-only displacement plus a frost read as a smear).
const LENS_ZOOM: f32 = 1.35;
// Extra radial displacement at the rim, on top of the zoom, as a fraction of the distance.
const LENS_DISP: f32 = 0.16;
// Drop-shadow strength under either shape, and how far down it sits (output px).
const LENS_SHADOW: f32 = 0.25;
const LENS_DROP: f32 = 2.0;
// The click ink drop's peak coverage, in the accent colour.
const LENS_INK: f32 = 0.30;
// The back's rim highlight (the sprite lens gets its rim from the pack's own artwork).
const BACK_RIM: f32 = 0.30;

// Signed distance to a rounded rect - the ONE primitive the back uses for all three of its shapes
// (disc, vertical pill, selection bar), which is what lets `back_geom` morph between them by lerp.
fn lens_rr_sd(p: vec2<f32>, mn: vec2<f32>, mx: vec2<f32>, r: f32) -> f32 {
  let q = abs(p - (mn + mx) * 0.5) - ((mx - mn) * 0.5 - vec2<f32>(r, r));
  return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// The sprite mask's coverage at `px`: the pack's own alpha, with the busy rotation undone about the
// box centre. 0 outside the box. `textureSampleLevel`, not `textureSample`: this is called from
// inside non-uniform control flow, where implicit derivatives are not allowed.
fn lens_cov(px: vec2<f32>) -> f32 {
  let half = max(u.lens_a.zw * 0.5 * u.lens_b.z, vec2<f32>(0.5, 0.5));
  let d = px - u.lens_a.xy;
  let s = sin(-u.lens_b.x);
  let k = cos(-u.lens_b.x);
  let r = vec2<f32>(d.x * k - d.y * s, d.x * s + d.y * k);
  let uv = r / half * 0.5 + vec2<f32>(0.5, 0.5);
  if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) { return 0.0; }
  return textureSampleLevel(lens_mask, samp, uv, 0.0).r;
}

// The glass itself: the frame re-sampled as a magnifier - a pixel shows what lies `1/LENS_ZOOM` of
// its distance from the centre, so everything inside is uniformly bigger and readable, with the rim
// pulled a little further in (`LENS_DISP`, strongest at the edge) so the edge still bends like
// glass. Frost only at the rim: the four taps close up to one crisp sample at the centre. Lifted
// ~8% and cooled.
fn lens_glass(px: vec2<f32>, c: vec2<f32>, half: vec2<f32>) -> vec3<f32> {
  let dims = vec2<f32>(u.a.x, u.a.y);
  let h = max(half, vec2<f32>(1.0, 1.0));
  let ref_ = max(min(h.x, h.y), 1.0);
  let d = px - c;
  let rr = clamp(length(d / h), 0.0, 1.0);
  let sp = c + d * ((1.0 - LENS_DISP * rr * rr) / LENS_ZOOM);
  let o = max(ref_ * 0.06, 0.7) * rr * rr;
  var col = textureSampleLevel(frame_tex, samp, sp / dims, 0.0).rgb * 0.40;
  col = col + textureSampleLevel(frame_tex, samp, (sp + vec2<f32>(o, 0.0)) / dims, 0.0).rgb * 0.15;
  col = col + textureSampleLevel(frame_tex, samp, (sp - vec2<f32>(o, 0.0)) / dims, 0.0).rgb * 0.15;
  col = col + textureSampleLevel(frame_tex, samp, (sp + vec2<f32>(0.0, o)) / dims, 0.0).rgb * 0.15;
  col = col + textureSampleLevel(frame_tex, samp, (sp - vec2<f32>(0.0, o)) / dims, 0.0).rgb * 0.15;
  return col * vec3<f32>(1.05, 1.08, 1.14);
}

// The ink drop: the accent colour spreading from the click point, easing out over its 260 ms and
// fading as it goes. `reach` is how far it gets - the shape's own diagonal.
fn lens_ink(col: vec3<f32>, px: vec2<f32>, at: vec2<f32>, p: f32, reach: f32) -> vec3<f32> {
  if (p < 0.0) { return col; }
  let r = max(fx_ease(clamp(p, 0.0, 1.0)) * reach, 0.5);
  let cov = 1.0 - smoothstep(r * 0.55, r, distance(px, at));
  return mix(col, u.color.rgb, LENS_INK * cov * (1.0 - clamp(p, 0.0, 1.0)));
}

// The cursor back, under everything: a glass disc / pill / selection bar with the same recipe, its
// own soft shadow, and either an ink drop or - over text - a thin ring hugging the pill.
//
// It carries a RIM the sprite lens does not need: a glass pack's artwork paints its own edge, and a
// bare disc without one has nothing to catch the light - over a flat background the refraction
// alone is a few percent of lift, i.e. a grey smudge. The rim plus the shadow are what make it read
// as an object with a thickness.
fn lens_back(base: vec3<f32>, px: vec2<f32>) -> vec3<f32> {
  if (u.back_b.y < 0.5) { return base; }
  let c = (u.back_a.xy + u.back_a.zw) * 0.5;
  let half = (u.back_a.zw - u.back_a.xy) * 0.5 * u.back_b.z;
  let r = u.back_b.x * u.back_b.z;
  let sd = lens_rr_sd(px, c - half, c + half, r);
  let m = clamp(0.5 - sd, 0.0, 1.0);
  // A bigger shape needs its shadow further out, or a 60 px disc looks pasted flat on the frame.
  let drop = max(LENS_DROP, min(half.x, half.y) * 0.10);
  let ssd = lens_rr_sd(px - vec2<f32>(0.0, drop), c - half, c + half, r);
  var color = base * (1.0 - LENS_SHADOW * clamp(0.5 - ssd / (drop * 0.9), 0.0, 1.0) * (1.0 - m));
  if (m <= 0.004) { return color; }
  var g = lens_glass(px, c, half);
  // The rim: a thin bright band just inside the edge, brightest along the top-left (where the
  // light is, matching the shadow's down-right offset).
  let lit = 0.6 + 0.4 * clamp(dot(normalize(c - px + vec2<f32>(0.0001, 0.0)),
                                  normalize(vec2<f32>(0.7, 0.7))), 0.0, 1.0);
  g = g + vec3<f32>(1.0, 1.0, 1.0) * clamp(1.0 - abs(sd + 1.2) / 1.6, 0.0, 1.0) * BACK_RIM * lit;
  if (u.back_c.z > 0.5) {
    let p = clamp(u.back_b.w, 0.0, 1.0);
    let band = clamp(1.0 - abs(sd + 1.5) / 1.5, 0.0, 1.0);
    if (u.back_b.w >= 0.0) { g = mix(g, u.color.rgb, band * (1.0 - p) * 0.8); }
  } else {
    g = lens_ink(g, px, u.back_c.xy, u.back_b.w, length(half) * 1.2);
  }
  // Fully replaced inside: a share of the un-magnified frame left showing through (it used to be
  // 15%) is a ghost of every letter under the zoomed one, which is what made the text unreadable.
  return mix(color, g, m);
}

// Both glass shapes, in the order they stack: the back, then the sprite-shaped lens the pack's own
// pixels land on. Called last in `fs_main`, after the click styles.
fn lens_fx(base: vec3<f32>, px: vec2<f32>) -> vec3<f32> {
  var color = lens_back(base, px);
  if (u.lens_b.y < 0.5) { return color; }
  let half = u.lens_a.zw * 0.5 * u.lens_b.z;
  let m = lens_cov(px);
  // The drop shadow is the mask itself, offset down and smeared over five taps; where the cursor
  // covers it (`1 - m`) it is hidden, so only the sliver that peeks out ever shows.
  var sh = lens_cov(px - vec2<f32>(0.0, LENS_DROP)) * 0.40;
  sh = sh + lens_cov(px - vec2<f32>(1.5, LENS_DROP)) * 0.15;
  sh = sh + lens_cov(px - vec2<f32>(-1.5, LENS_DROP)) * 0.15;
  sh = sh + lens_cov(px - vec2<f32>(0.0, LENS_DROP - 1.5)) * 0.15;
  sh = sh + lens_cov(px - vec2<f32>(0.0, LENS_DROP + 1.5)) * 0.15;
  color = color * (1.0 - LENS_SHADOW * sh * (1.0 - m));
  if (m <= 0.004) { return color; }
  let g = lens_ink(lens_glass(px, u.lens_a.xy, half), px, u.lens_c.xy, u.lens_b.w, length(half) * 1.2);
  return mix(color, g, m);
}
