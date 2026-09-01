# src/editor/stage/spotlightPreview.ts

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

## SpotlightSimState

```ts
export interface SpotlightSimState {
  driver: string | null; // the winning EffectRegion's id, or null
  transitionFrom: number | null;
  transitionStart: number | null;
  transitionDur: number | null;
  alpha: number;
}
```

The persistent, per-frame-loop state `resolveSpotlight` reads and mutates in place, mirroring the export's `SpotlightSim` (`spotlight_sim.rs`): which region is the current "driver" (winner), and - while easing across a handoff - the alpha/time snapshot the transition started from and how long it runs. Owned by the caller (`useCompositeLoop` keeps one in a ref, `newSpotlightSimState()`-initialized) and threaded into every `resolveSpotlight` call for the loop's whole lifetime, so the handoff easing has continuity across frames.

## newSpotlightSimState

```ts
export function newSpotlightSimState(): SpotlightSimState
```

Returns a fresh, idle `SpotlightSimState` (`driver: null`, no transition in progress, `alpha: 0`).

## resolveSpotlight

```ts
export function resolveSpotlight(spotlight: SpotlightInput, ms: number, sim: SpotlightSimState): ResolvedSpotlight | null
```

Resolves the spotlight at output time `ms`, or `null` when it's fully off (`alpha <= 0`). **Must be called exactly once per composite tick** - it mutates `sim` in place, so a second call at the same `ms` would double-advance the same stateful transition; `useCompositeLoop` calls it once and reuses the result for both the FX-overlay cache key and the request itself (`fxOverlay.ts`'s `requestFxOverlay` takes the already-resolved value, not the raw inputs).

### Inputs

- `spotlight: SpotlightInput` - the regions, toggle, and base params.
- `ms: number` - the output time to resolve at.
- `sim: SpotlightSimState` - the persistent handoff/transition state, read and mutated in place.

### Returns

`ResolvedSpotlight | null` - `null` when nothing should be drawn (skips the backend request entirely).

### Implementation

1. Find the winning region at `ms` (highest `layer`, ties broken later-in-array, matching Rust's `max_by_key`). If the winner changed since the last call (`win?.id !== sim.driver`) **and** a region was previously driving (`sim.driver !== null`), arm a transition: snapshot the current `sim.alpha` as `transitionFrom`, `ms` as `transitionStart`, and the incoming winner's `fade_in_ms` (or, on exit, the outgoing region's `fade_out_ms`) as `transitionDur`. Entering a region from "no driver" does **not** arm one - this matches Rust's identical `driver.is_some()` guard, so a transition armed on a previous exit can still be in effect when a new region starts driving.
2. `natural` = the winning region's own fade-in/out alpha at `ms` (0 if no winner).
3. If a transition is in progress: `elapsed = max(0, ms - transitionStart)` - **clamped at 0**, mirroring Rust's `et.saturating_sub(tr.start_ms)`. A backwards `ms` (scrubbing before `transitionStart`) can only ever mean "the transition hasn't started yet" and holds `sim.alpha` at `transitionFrom`; before the clamp, a signed `elapsed` let `e = elapsed / transitionDur` go negative, and the smoothstep `e*e*(3-2e)` is unbounded below zero (e.g. `e=-8` → `1216`), driving `alpha` far outside 0..1 - and it never cleared, since `elapsed < transitionDur` stayed true forever for a negative `elapsed`. Once `elapsed >= transitionDur`, the transition clears and `sim.alpha = natural`.
4. `alpha = max(sim.alpha, on ? 1 : 0)` - mirrors the export's toggle-union-with-region behavior. Returns `null` if `alpha <= 0`.
5. Per-field overrides: `mode`/`dim`/`radius`/`feather` come from the active region when it sets one (and, for `mode`, isn't `"global"`), else fall back to `params`. `tint` always comes from `params` (not per-region yet).

### Notes

- Called once per tick from `useCompositeLoop`, which reuses the result both to build the FX-overlay cache key and (passed straight through) as `fxOverlay.ts`'s `requestFxOverlay` `resolved` param - not called from inside `requestFxOverlay` itself.
