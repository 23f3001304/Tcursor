# src-tauri/src/export/fx/clickdraw.rs

Renders all click effect styles (ripple, pulse, glow, neon, shockwave, particles) onto a BGRA frame buffer by dispatching per-hit draws from `FxState`. Each style is built from private geometric primitives (ring, feathered disc, gaussian bloom, additive ring, capsule streak). All sizes are expressed as fractions of frame height `oh` so effects scale consistently with resolution. Invoked by `fxdraw::CpuFx::apply` as the topmost compositing layer.

**`fx_clicks.wgsl` is the reference look.** Every radius, thickness, alpha curve and additive gain here mirrors it; when the two disagree, this file is wrong. The shared timing (`ease_out`, `fade_alpha`) comes from `clickfx.rs`, which is the same curve the shader's `fx_ease`/`fx_alpha` compute. Two things cannot be mirrored:

- **Shockwave's refraction and chromatic dispersion.** The shader warps the sampled UV across the band and pulls R and B from 2 px either side of the displacement (`fx.wgsl`, before the texture sample). This path composites in place with no source to re-sample, so Shockwave here is its ring and white rim alone. The ring's `0.5` gain is deliberately understated for exactly that reason in the shader; it stays understated here rather than being "compensated", so the two paths agree numerically.
- **`Particles`' exact spark placement.** The shader's `ck_hash` is `fract(sin(x * 127.1) * 43758.5453)`, and because the GPU's `sin` is an approximation while this chaotic hash amplifies any difference, individual sparks land on different pixels. What is mirrored is the distribution and the drawing: 14 pseudo-random angles, per-spark speeds, and a streak back along each spark's own velocity.

## hash1

```rust
fn hash1(x: f32) -> f32
```

`fx_clicks.wgsl`'s `ck_hash`, ported for `particles`. Returns a pseudo-random `0..1` from a small integer input. *Why not a real RNG:* the sparks must be identical for the same click on every frame the effect is alive (a stateless function of the spark index), and matching the shader's formula keeps the two bursts statistically identical even though they cannot be pixel-identical.

### Used by

- `src-tauri/src/export/fx/clickdraw.rs` (`particles`) - one call for the spark's angle, one for its speed.

## for_seg

```rust
fn for_seg(ow: u32, oh: u32, a: (f32, f32), b: (f32, f32), rad: f32, f: impl FnMut(usize, f32))
```

Visits every pixel within `rad` of the segment `a`..`b`, handing the callback the pixel's byte index and its distance to that segment - the CPU half of the shader's `ck_seg`, which is how a particle becomes a streak instead of a dot.

### Implementation

Walks the bounding disc of the capsule (`for_disc` at the segment's midpoint with radius `half_length + rad + 1`), recovers each pixel's `(x, y)` from its byte index, and projects it onto the segment with the clamped parameter `t = dot(p - a, b - a) / |b - a|^2`. *Why the bounding disc rather than a rectangle walk:* `for_disc` already exists, already clips to the frame, and the wasted corners of a short capsule are a handful of pixels.

## flash

```rust
fn flash(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, p: f32, inten: f32)
```

The impact flash every style shares: an additive white gaussian bloom of radius `oh * 0.015` at the click point, scaled by `(1 - smoothstep(0, 0.14, p)) * intensity` so it is gone about 80 ms into the 600 ms life.

*Why every style gets it:* the click has to read as landing at a point before the style's own geometry has had time to grow. Without it the first two or three frames of a ring or a disc are a barely-visible speck and the effect appears to start late.

## soft_disc

```rust
fn soft_disc(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, c: [u8; 3], a: f32)
```

Pulse's body: alpha-blended, solid out to `radius * 0.2`, then smoothstepping away to nothing at `radius`.

*Why the feather starts that early:* a disc that stays solid most of the way out reads as a flat sticker pasted on the frame - which is what the old fixed-radius Pulse looked like in the bench renders. Feathered from a fifth of the radius it is a bloom with a rim.

## disc_add

```rust
fn disc_add(out: &mut [u8], ow: u32, oh: u32, cx: f32, cy: f32, radius: f32, c: [u8; 3], a: f32)
```

A small additive disc with a 1 px antialiased edge (`clamp(radius - d, 0, 1)`), mirroring the shader's `clamp(oh * 0.006 - d, 0, 1)` - Pulse's white core.

## draw_clicks

```rust
pub fn draw_clicks(out: &mut [u8], ow: u32, oh: u32, state: &FxState)
```

Iterates over all active click hits in `state` and renders each one using the style and colour from `state`.

### Inputs

- `out: &mut [u8]` - BGRA frame buffer (`ow * oh * 4` bytes). *Why mutable:* all effects blend or add onto the existing pixels in place; no second buffer is allocated.
- `ow: u32` - output frame width. *Why:* used for per-pixel index computation and for computing the blit bounds in `for_disc`.
- `oh: u32` - output frame height. *Why:* all size constants are expressed as fractions of `oh` so the same settings look the same at 720p, 1080p, and 4K.
- `state: &FxState` - the current effect snapshot: `state.style` selects the drawing algorithm; `state.color` is the RGB value applied to all hits; `state.intensity` is the alpha scale; `state.hits` contains all live hits with their output-space position `(x, y)` and normalised `progress` 0..1. *Why a single state struct:* keeps the function signature stable as new style variants are added to `ClickFxStyle`.

### Returns

`()`. Returns immediately with no work when `state.style` is `ClickFxStyle::None`.

### Implementation

1. If `state.style == ClickFxStyle::None`, return immediately.
2. For each `h` in `state.hits`, take `p = h.progress` clamped and `a = fade_alpha(p, state.intensity)`, then draw `flash` first (every style), then the style:
   - `Ripple` - a gaussian halo at twice the lead ring's radius (gain `a * 0.15`), then THREE alpha-blended rings launched 0 / 90 / 180 ms apart (progress offsets 0, 0.15, 0.30, each existing only once its offset has passed), radii `ripple_radius(p - offset, oh * 0.06)`, thicknesses `oh * 0.006 / 0.0045 / 0.003`, gains `a * 1.0 / 0.7 / 0.45`. *Why three:* one ring is a widget; a lead ring with two fainter trails reads as a surface responding.
   - `Pulse` - `soft_disc` growing `oh * 0.01` -> `oh * 0.035` (eased), an additive tint rim on its edge (thickness `oh * 0.004`, gain `a * 0.6`), and `disc_add` of white at `oh * 0.006` faded out by `p = 0.3`. *Why a rim:* it gives the bloom a defined edge, which is what separates a pulse from a smudge.
   - `Glow` - the gaussian bloom at `oh * 0.05 * (0.6 + 0.8 * p)`, its radius shimmering by `1 + 0.06 * sin(4 pi p)` (two cycles over the life), plus a hot white core bloom at `oh * 0.008`, gain `a * 0.5`.
   - `Neon` - TWO tubes. The first at `ease_out(p) * oh * 0.07`, thickness `oh * 0.01`: the tint at gain `a * 1.4`, then white at `a * 0.25`. The second launches at `p = 0.12` (72 ms later) at thickness `oh * 0.008`, drawn in `hue_shift(color, NEON_HUE_SHIFT)` at gain `a`, then white at `a * 0.18`. *Why the white passes:* they are what blow each tube's core out to near-white, which is the reason neon reads as *neon* rather than as a plain bright ring. *Why the gains are not capped with `min(.., 1.0)`:* the shader clamps only the final colour, so a gain above 1 is meant to saturate the channel; capping the weight instead made the ring visibly dimmer than the export's.
   - `Shockwave` - the tinted ring at `ease_out(p) * oh * 0.09`, thickness `oh * 0.008`, gain `a * 0.5`, plus a white leading rim `oh * 0.004` outside it (thickness `oh * 0.004`, gain `a * 0.35`). The shader's refraction and dispersion have no CPU counterpart (see the note at the top).
   - `Particles` - `particles`: 14 sparks at `hash1`-derived angles with per-spark speeds `(0.4 + hash1(k + 7)) * oh * 0.10`, gravity `oh * 0.06 * p^2`, each drawn as a capsule from `pos - velocity * 0.06` to `pos` and faded by `a * (1 - p^2)`. *Why streaks rather than dots:* a dot burst reads as confetti; a streak along the spark's own velocity reads as a spark with direction.
