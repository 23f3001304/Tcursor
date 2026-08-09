# src-tauri/src/export/fx/clickdraw.rs

Renders all click effect styles (ripple, pulse, glow, neon, shockwave, particles) onto a BGRA frame buffer by dispatching per-hit draws from `FxState`. Each style uses a private geometric primitive (ring, disc, gaussian glow, additive ring, or scattered sparks). All sizes are expressed as fractions of frame height `oh` so effects scale consistently with resolution. Invoked by `fxdraw::CpuFx::apply` as the topmost compositing layer.

**`fx.wgsl`'s click block is the reference look.** Every radius, thickness, alpha curve and additive gain here mirrors it; when the two disagree, this file is wrong. The one thing that cannot be mirrored is `Particles`' exact spark placement - the shader's `hash1` is `fract(sin(x * 127.1) * 43758.5453)`, and because the GPU's `sin` is an approximation while this chaotic hash amplifies any difference, individual sparks land on different pixels. What is mirrored is the distribution: pseudo-random angles and per-spark speeds (a burst) rather than the evenly-spaced 12-spoke ring this file drew before.

## hash1

```rust
fn hash1(x: f32) -> f32
```

`fx.wgsl`'s one-dimensional hash, ported for `particles`. Returns a pseudo-random `0..1` from a small integer input. *Why not a real RNG:* the sparks must be identical for the same click on every frame the effect is alive (a stateless function of the spark index), and matching the shader's formula keeps the two bursts statistically identical even though they cannot be pixel-identical.

### Used by

- `src-tauri/src/export/fx/clickdraw.rs` (`particles`) - one call for the spark's angle, one for its speed.

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

`()`. Returns immediately with no work when `state.style` is `ClickFxStyle::None`. *Why early return:* when effects are disabled the per-hit loop is skipped entirely.

### Implementation

1. If `state.style == ClickFxStyle::None`, return immediately.
2. Compute `r_max = oh * 0.06` (the ripple's maximum ring radius).
3. For each `h` in `state.hits`:
   - Compute `a = fade_alpha(h.progress, state.intensity)` as the base alpha for this hit.
   - Dispatch on `state.style`:
     - `Ripple` - `ring` at `ripple_radius(h.progress, r_max)`, thickness `oh * 0.006`, alpha-blend. *Why growing radius at fixed alpha-blend:* mimics a water ripple expanding from the click point.
     - `Pulse` - `disc` (filled circle) at fixed radius `oh * 0.02`, alpha-blend. *Why fixed radius:* a steady disc at the click point that simply fades is a quick, minimal acknowledgement.
     - `Glow` - `glow` (gaussian bloom) at radius `oh * 0.05 * (0.6 + 0.8 * h.progress)`; radius grows with progress. *Why growing radius for glow:* matches the feel of a bloom that expands and fades simultaneously.
     - `Neon` - TWO additive rings at the same expanding radius `h.progress * oh * 0.07` and thickness `oh * 0.01`: the tint at gain `a * 1.4`, then white at `a * 0.25`. *Why two passes:* this is exactly what `fx.wgsl` does (`u.color.rgb * cov * a * 1.4 + vec3(1) * cov * a * 0.25`); the white pass is what blows the ring's core out to near-white and is the reason neon reads as *neon* rather than as a plain bright ring. *Why the gain is no longer capped with `min(.., 1.0)`:* the shader clamps only the final colour, so a gain above 1 is meant to saturate the channel; capping the weight instead made the ring visibly dimmer than the export's.
     - `Shockwave` - `ring_add` at `h.progress * oh * 0.09` (wider travel), thickness `oh * 0.008` (thinner line), gain `a * 0.5`. *Why wider and fainter than Neon:* shockwave expands faster; in the shader the ring is only half the effect - it also warps the sampled UVs radially around the ring (a refraction ripple this CPU path cannot reproduce, since it composites in place with no source to re-sample), so the ring itself is deliberately understated at `0.5`.
     - `Particles` - `particles`, 12 sparks at `hash1`-derived angles with per-spark speeds `(0.4 + hash1(k + 7)) * oh * 0.10`. *Why hashed rather than evenly spaced:* the shader's burst is irregular; 12 sparks at `k / 12 * TAU` produced a perfectly symmetric 12-spoke asterisk instead, which was the most visible click-FX difference between preview and export.
