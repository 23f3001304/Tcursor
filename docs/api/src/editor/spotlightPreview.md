# src/editor/spotlightPreview.ts

Resolves the spotlight's strength and look at a given output time, mirroring the export's `fx_state::fx_state_at` so the editor preview requests the same spotlight the backend would render. Drawing itself happens entirely in the backend (`preview_fx.rs`'s shader/CPU pipeline, requested via `fxOverlay.ts`) for pixel-perfect parity - this file only resolves *what* to ask for.

## SpotParams

```ts
export interface SpotParams {
  dim: number; radius: number; feather: number; mode: string; tint: [number, number, number];
}
```

The spotlight look from the recording's `clickfx` settings: `dim` (max darkness 0..1), `radius`/`feather` (fractions of output height), `mode` (Classic/Blur/Halo/Breathing/Nebula/Vignette), and `tint` (RGB, used by Halo).

## SpotlightInput

```ts
export interface SpotlightInput { effects: EffectRegion[]; on: boolean; params: SpotParams }
```

What `resolveSpotlight` needs: the editable Spotlight `EffectRegion`s, the global settings toggle (`on`), and the base params. `useCompositeLoop` builds one every frame from `effectsRef`/`clickfxRef`.

## ResolvedSpotlight

```ts
export interface ResolvedSpotlight {
  alpha: number; mode: string; dim: number; radius: number; feather: number; tint: [number, number, number];
}
```

The spotlight state at one instant: `alpha` (0 = fully off, skip the request) and the effective `mode`/`dim`/`radius`/`feather`/`tint` after any active region's per-field overrides are applied.

## resolveSpotlight

```ts
export function resolveSpotlight(spotlight: SpotlightInput, ms: number): ResolvedSpotlight | null
```

Resolves the spotlight at output time `ms`, or `null` when it's fully off (`alpha <= 0`).

### Inputs

- `spotlight: SpotlightInput` - the regions, toggle, and base params.
- `ms: number` - the output time to resolve at.

### Returns

`ResolvedSpotlight | null` - `null` when nothing should be drawn (skips the backend request entirely).

### Implementation

1. `settingsAlpha = on ? 1 : 0`.
2. Find the Spotlight region active at `ms` (`start_ms <= ms < end_ms`); if found, ramp its alpha in/out over `fade_in_ms`/`fade_out_ms` (clamped 0..1).
3. `alpha = max(settingsAlpha, regionAlpha)` - mirrors the export's toggle-union-with-region behavior. Returns `null` if `alpha <= 0`.
4. Per-field overrides: `mode`/`dim`/`radius`/`feather` come from the active region when it sets one (and, for `mode`, isn't `"global"`), else fall back to `params`. `tint` always comes from `params` (not per-region yet).

### Notes

- Called from `fxOverlay.ts`'s `requestFxOverlay` to decide whether to skip the backend request entirely and to build the `spotAlpha`/params sent to `preview_fx_overlay`.
