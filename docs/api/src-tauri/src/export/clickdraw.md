# src-tauri/src/export/clickdraw.rs

Renders all click effect styles (ripple, pulse, glow, neon, shockwave, particles) onto a BGRA frame buffer by dispatching per-hit draws from `FxState`. Each style uses a private geometric primitive (ring, disc, gaussian glow, additive ring, or scattered sparks). All sizes are expressed as fractions of frame height `oh` so effects scale consistently with resolution. Invoked by `fxdraw::CpuFx::apply` as the topmost compositing layer.

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
     - `Neon` - `ring_add` (additive ring) with expanding radius `h.progress * oh * 0.07`, thickness `oh * 0.01`, alpha `min(a * 1.4, 1.0)`. *Why additive and boosted alpha:* additive blending brightens whatever is beneath, giving a neon-on-dark-surface look without being bounded by the existing pixel brightness.
     - `Shockwave` - `ring_add` at `h.progress * oh * 0.09` (wider travel), thickness `oh * 0.008` (thinner line), alpha `min(a * 0.7, 1.0)`. *Why wider and lower alpha than Neon:* shockwave expands faster and looks more subtle; the dimmer alpha prevents it from washing out the frame.
     - `Particles` - `particles` with `spread = oh * 0.10` and 12 sparks distributed evenly around TAU. *Why 12 sparks:* divisor chosen so no two sparks visually overlap at typical sizes; spread as a fraction of `oh` keeps sparks appropriately sized at any resolution.
