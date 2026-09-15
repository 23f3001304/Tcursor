# src-tauri/src/export/fx/spot/mod.rs

Submodule overviews for the spotlight group: which region is lit and how strongly (`spotlight_sim`), the hold-key adapter that lets a chord light one up live (`spotlight`), and the CPU pass that darkens everything outside it (`spotdraw`). The GPU path draws the same thing from `fx.wgsl` instead.

## spotlight_sim

Restricted to the `fx` group (`pub(super) mod spotlight_sim;`). The stateful spotlight region resolver: `fx_state.rs` owns the per-frame state, this file owns the one thing in it that remembers the previous frame, and re-exports it as `fx_state::SpotlightSim`. Key items: `region_alpha(region, et) -> f32` (one region's own fade ramp), `SpotlightSim` (tracks the highest-layer active region and eases alpha across a handoff), `SpotlightSim::resolve(effects, et, settings_on) -> f32`, `SpotlightSim::style(effects, fx) -> (SpotlightMode, f32, f32, f32)`. See `spot/spotlight_sim.md`.

## spotlight

Thin typed adapter over `hold::hold_alpha` for the spotlight effect. Key items: `hold_alpha(actions, et, fade_ms)` - returns spotlight strength 0..1 ramping around `SpotlightHoldStart`/`SpotlightHoldEnd` action pairs. See `spot/spotlight.md`.

## spotdraw

Renders the spotlight effect in six modes (Classic, Breathing, Vignette, Blur, Nebula, Halo) by darkening pixels outside the lit zone. Key items: `draw_spot(out, ow, oh, s)` - modifies `out` in-place, skips the double loop entirely when `s.dim * s.alpha <= 0`. See `spot/spotdraw.md`.
