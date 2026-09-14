// Click-effect styles for fx.wgsl. `fx_gpu.rs::build_pipeline` concatenates this file AFTER
// fx.wgsl into one shader module, so the uniform `u`, the `FX_*` style ids and the `FxU` struct
// all come from there; only the click code lives here (both files then fit the 200-line budget).
// `clickdraw.rs` mirrors this geometry on the CPU, `ripplePreview.ts` mirrors ripple/shockwave/
// pulse in the editor preview. When the three disagree, THIS file is right.

// Shared timing (mirrored by clickfx.rs `ease_out`/`fade_alpha`, ripplePreview.ts `easeOut`/
// `rippleAlpha`): radii ease out so an effect leaves the click fast and settles, alpha holds for
// the first 55% of the life then releases - a hit, not a linear dissolve.
fn fx_ease(p: f32) -> f32 { let q = 1.0 - clamp(p, 0.0, 1.0); return 1.0 - q * q * q; }
fn fx_alpha(p: f32) -> f32 { return 1.0 - smoothstep(0.55, 1.0, clamp(p, 0.0, 1.0)); }

fn ck_hash(x: f32) -> f32 { return fract(sin(x * 127.1) * 43758.5453); }

// Triangular ring coverage: 1 on the ring, 0 a full `thick` off it.
fn ck_ring(d: f32, radius: f32, thick: f32) -> f32 {
  let t = max(thick, 1.0);
  return clamp((t - abs(d - radius)) / t, 0.0, 1.0);
}
// Gaussian bloom falloff at distance `d` for a bloom of radius `r`.
fn ck_gauss(d: f32, r: f32) -> f32 { let s = max(r, 1.0); return exp(-(d * d) / (s * s)); }
// Distance from `p` to the segment a..b - the spine of a particle streak's capsule.
fn ck_seg(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = clamp(dot(pa, ba) / max(dot(ba, ba), 0.0001), 0.0, 1.0);
  return length(pa - ba * h);
}

// Every click style, added on top of the already-composited `base` colour. `px` is this
// fragment in output pixels, `oh` the output height (every size is a fraction of it, so the look
// is resolution-independent), `style` the `FX_*` id and `n` the live hit count.
fn clicks(base: vec3<f32>, px: vec2<f32>, oh: f32, style: f32, n: i32) -> vec3<f32> {
  var color = base;
  if (style < 0.5) { return color; }
  let inten = clamp(u.c.z, 0.0, 1.0);
  let tint = u.color.rgb;
  let white = vec3<f32>(1.0, 1.0, 1.0);
  for (var i = 0; i < n; i = i + 1) {
    let h = u.hits[i];
    let p = clamp(h.z, 0.0, 1.0);
    let a = fx_alpha(p) * inten;
    let d = distance(px, h.xy);
    // Impact flash: a soft white bloom at the point for the first ~80 ms (0.14 of the 600 ms
    // life). Every style gets it - it is what makes the effect read as "the click landed here"
    // before its own geometry has had time to grow.
    color = color + white * ck_gauss(d, oh * 0.015) * (1.0 - smoothstep(0.0, 0.14, p)) * inten;
    if (style == FX_RIPPLE) {
      // Three rings launched 0 / 90 / 180 ms apart (progress offsets 0, 0.15, 0.30), each
      // thinner and fainter, over a soft halo at twice the lead ring's radius.
      let r1 = fx_ease(p) * oh * 0.06;
      color = color + tint * ck_gauss(d, r1 * 2.0) * a * 0.15;
      color = mix(color, tint, a * ck_ring(d, r1, oh * 0.006));
      let p2 = p - 0.15;
      if (p2 > 0.0) { color = mix(color, tint, a * 0.7 * ck_ring(d, fx_ease(p2) * oh * 0.06, oh * 0.0045)); }
      let p3 = p - 0.30;
      if (p3 > 0.0) { color = mix(color, tint, a * 0.45 * ck_ring(d, fx_ease(p3) * oh * 0.06, oh * 0.003)); }
    } else if (style == FX_PULSE) {
      // A feathered disc growing 1% -> 3.5% of height, a rim on its edge at 60% alpha, and a
      // bright white core that is gone by p = 0.3.
      // The feather starts at a fifth of the radius, not at half of it: a disc that stays solid
      // most of the way out reads as a flat sticker pasted on the frame, which is exactly what
      // the old fixed-radius pulse looked like. Feathered this early it is a bloom with a rim.
      let r = oh * (0.01 + 0.025 * fx_ease(p));
      color = mix(color, tint, a * (1.0 - smoothstep(r * 0.2, r, d)));
      color = color + tint * ck_ring(d, r, oh * 0.004) * a * 0.6;
      color = color + white * clamp(oh * 0.006 - d, 0.0, 1.0) * a * (1.0 - smoothstep(0.0, 0.3, p));
    } else if (style == FX_GLOW) {
      // The bloom, shimmering (two cycles over the life), around a hot white core.
      let r = oh * 0.05 * (0.6 + 0.8 * p) * (1.0 + 0.06 * sin(p * 12.566371));
      color = color + tint * ck_gauss(d, r) * a;
      color = color + white * ck_gauss(d, oh * 0.008) * a * 0.5;
    } else if (style == FX_NEON) {
      // Two tubes: the tint, then a hue-rotated one (u.color2, rotated 30 degrees in Rust)
      // launched 72 ms later, each with the blown-out white core that makes neon read as neon.
      let c1 = ck_ring(d, fx_ease(p) * oh * 0.07, oh * 0.01);
      color = color + tint * c1 * a * 1.4 + white * c1 * a * 0.25;
      let p2 = p - 0.12;
      if (p2 > 0.0) {
        let c2 = ck_ring(d, fx_ease(p2) * oh * 0.07, oh * 0.008);
        color = color + u.color2.rgb * c2 * a + white * c2 * a * 0.18;
      }
    } else if (style == FX_SHOCKWAVE) {
      // The band's refraction + dispersion happens in fx.wgsl (it has to warp the SAMPLE);
      // here it gets its line: the tinted ring plus a hot white rim just outside it.
      let r = fx_ease(p) * oh * 0.09;
      color = color + tint * ck_ring(d, r, oh * 0.008) * a * 0.5;
      color = color + white * ck_ring(d, r + oh * 0.004, oh * 0.004) * a * 0.35;
    } else if (style == FX_PARTICLES) {
      // 14 sparks at hashed angles, each drawn as a streak along its own velocity (a capsule
      // back over the last 0.06 of progress) so the burst has direction instead of dots.
      let pa = a * (1.0 - p * p);
      for (var k = 0; k < 14; k = k + 1) {
        let ang = ck_hash(f32(k)) * 6.2831853;
        let sp = (0.4 + ck_hash(f32(k) + 7.0)) * oh * 0.10;
        let dir = vec2<f32>(cos(ang), sin(ang));
        let pos = h.xy + dir * sp * p + vec2<f32>(0.0, oh * 0.06 * p * p);
        let vel = dir * sp + vec2<f32>(0.0, oh * 0.12 * p);
        color = color + tint * clamp(oh * 0.004 - ck_seg(px, pos - vel * 0.06, pos), 0.0, 1.0) * pa;
      }
    }
  }
  return color;
}
