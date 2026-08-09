# src-tauri/src/export/fx/spotlight_sim.rs

The stateful spotlight region resolver, split out of `fx_state.rs` (which was at the 200-line budget). Re-exported as `fx_state::SpotlightSim` so existing callers are unaffected by the split; `region_alpha` stays internal (only `SpotlightSim::resolve` and this file's own tests use it directly).

## region_alpha

```rust
pub(crate) fn region_alpha(e: &EffectRegion, et: u32) -> f32
```

One region's own fade-in/out ramp at `et` (`0` outside its `[start_ms, end_ms)` span), independent of any other region - the building block `SpotlightSim` blends across a handoff.

### Implementation

`inn = (et - start_ms) / fade_in_ms`, `outn = (end_ms - et) / fade_out_ms`, result is `min(inn, outn)` clamped to `[0, 1]` - so alpha ramps up over `fade_in_ms`, holds at 1 in the middle (as long as the region is longer than both fades combined), and ramps down over `fade_out_ms`.

### Behaviors

- `spotlight_uses_per_region_fades` - 50 ms into a 100 ms fade-in is ~0.5; 250 ms before the end of a 500 ms fade-out is ~0.5.

## SpotlightSim

```rust
#[derive(Default)]
pub struct SpotlightSim { driver: Option<usize>, transition: Option<SpotTransition>, alpha: f32 }
```

Stateful spotlight resolver, mirroring `CameraSim`'s pattern: tracks which Spotlight `EffectRegion` (by index into the caller's `effects` slice) is currently the highest-layer active one, and eases alpha across a handoff instead of jump-maxing across overlaps. Style (mode/dim/radius/feather) always comes from the current winner - no blending of two regions' looks simultaneously (override semantics, not compose). Carried across frames by the renderer (`FrameRenderer::spot_sim`) so the handoff easing has continuity.

## SpotlightSim::resolve

```rust
pub fn resolve(&mut self, effects: &[EffectRegion], et: u32, settings_on: bool) -> f32
```

Resolves alpha at `et` (the OUTPUT-clock region time, called `region_t` at the `fx_state_at` call site), unioned with the flat (non-transitioning) settings toggle.

### Implementation

1. `winner(effects, et)` finds the highest-`(layer, index)` active Spotlight region, or `None`.
2. If the winner changed since the last call AND a previous driver existed, start a `SpotTransition` from the current `alpha`, over the NEW winner's `fade_in_ms` (or the OLD winner's `fade_out_ms` when the new winner is `None`) - so a handoff eases through the incoming/outgoing region's own fade duration rather than a fixed constant.
3. Compute `natural = region_alpha(winner, et)` (or `0.0` with no winner).
4. If a transition is in flight and not yet elapsed, ease from `transition.from_alpha` toward `natural` via `ease(Easing::Smooth, ...)`; once elapsed, clear the transition and snap to `natural`.
5. `alpha.max(settings_on ? 1.0 : 0.0)` - the flat toggle is a floor, not an override, so a region fading out while the toggle is on doesn't visibly dip.

### Behaviors

- `highest_layer_region_wins_style_not_first_match` - two same-span regions, layer 1 (`b`) wins over layer 0 (`a`) regardless of iteration order.
- `spotlight_handoff_eases_alpha_instead_of_jump_maxing` - a layer-0 region fully faded in by t=500, then a layer-1 region takes over at t=500 with a slow 300 ms fade-in: alpha at t=520 stays near the layer-0 level (>0.8), not the layer-1 region's own barely-started fade (~0.07).

## SpotlightSim::style

```rust
pub fn style(&self, effects: &[EffectRegion], fx: &ClickFxSettings) -> (SpotlightMode, f32, f32, f32)
```

The current winner's `(mode, dim, radius, feather)`, falling back per-field to the global `ClickFxSettings` when the winning region leaves that field unset (`None`) or there is no winner at all.

### Used by

- `src-tauri/src/export/fx/fx_state.rs` - `fx_state_at` calls this after `resolve` to build the frame's `Spot`.
