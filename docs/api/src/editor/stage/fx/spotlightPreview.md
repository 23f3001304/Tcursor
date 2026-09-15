# src/editor/stage/fx/spotlightPreview.ts

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

## spotlightEffectsKey

```ts
export function spotlightEffectsKey(effects: EffectRegion[]): string
```

A cheap CONTENT signature for the spotlight-relevant fields of `effects` - `id`/`start_ms`/`end_ms`/`fade_in_ms`/`fade_out_ms`/`mode`/`dim`/`radius`/`feather` per entry, joined in array order (stable across array/object reference changes, sensitive to reordering, `layer`/`kind` not tracked). Mirrors `cameraMovesKey` (`cameraMoves.ts`) for the identical reason: `applyEditOp` round-trips the whole `EditDoc` through IPC, so `effects` gets a brand-new array reference on every edit routed through it (add a zoom, trim, an AI-director step), not just spotlight ones - keying a reset on the reference would reset on every unrelated edit.

`Stage.tsx` keeps a ref of the last key and resets its `spotSimRef` (passed into `useCompositeLoop`) in a `[effects]`-deps `useEffect` when the key changes - this is "gate 2" of the fix for the M9 "spotlight freezes mid-fade" bug: a Spotlight region retimed/added/removed **while paused** doesn't move `timeMs` at all, so `useCompositeLoop`'s discontinuous-jump reset (gate 1, in its tick loop) never fires for it. Without gate 2, a driver disappearing via that paused edit arms an outgoing fade transition in `resolveSpotlight` that never advances (its `elapsed` is computed from `ms`, which is frozen), so `sim.alpha` - and the dimmed spotlight overlay - would hold on screen indefinitely.

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

## ALPHA_LINEAR_SPOT_MODES

```ts
export const ALPHA_LINEAR_SPOT_MODES: ReadonlySet<string> = new Set(["classic", "breathing", "vignette"]);
```

The spotlight modes whose rendered output is **exactly proportional** to `Spot.alpha`, on both renderers. Each of these three is a pure multiplicative dim - `color * (1 - dim*alpha * t)` (fx.wgsl's `u.b.z` is that `dim*alpha` product; `spotdraw.rs`'s `k` is the same thing) - so the recovered straight-alpha overlay rendered at `alpha = 1` has per-pixel alpha `dim*t`, and compositing THAT at `ctx.globalAlpha = a` yields `dim*t*a`: bit-for-bit what the backend would have returned had it been asked for `alpha = a`. That equivalence is the entire licence for `spotAlphaPlan`, and it is pinned on the Rust side by `classic_spotlight_overlay_alpha_is_proportional_to_spot_alpha` (`preview_fx_alpha_tests.rs`), which runs the real `select_fx` renderer.

The other three are excluded because their alpha response is **not** proportional:

| Mode | Why not |
| --- | --- |
| `halo` | adds an `intensity`-scaled ring (`color + tint*band*intensity`) that never reads alpha at all. |
| `nebula` | the shader path never reads `u.b.z`; its dim is a fixed `1 - 0.80*t`. |
| `blur` | mixes toward a blurred sample by `t`, with alpha only trimming that sample's brightness - so at `alpha -> 0` the blur is still nearly full strength. |

Those three consequently barely fade in the **export** either, which is a real but separate renderer-side issue (see the report for 2026-09-02). Scaling them client-side would make the preview disagree with the export, so they keep the backend round-trip they have always used.

## SpotAlphaPlan

```ts
export interface SpotAlphaPlan { requestAlpha: number; drawAlpha: number; separable: boolean }
```

The three-way split `spotAlphaPlan` (below) produces; see its **Returns** for what each field means.

## spotAlphaPlan

```ts
export function spotAlphaPlan(resolved: ResolvedSpotlight | null, overlayHasClicks: boolean): SpotAlphaPlan
```

How one composite tick should split the spotlight's alpha between the **backend request** and the **client-side blit**.

*The bug this exists for:* unlike layout/camera (drawn per-frame in TS at 60fps), the spotlight is a backend-rendered PNG overlay. `useCompositeLoop` requests it at most every `FX_BUCKET_MS` (40ms) and single-flights it, and each request costs a GPU readback + full-frame PNG encode + base64 + an `Image` decode. A 250ms fade therefore got a handful of overlay updates at best - and whenever one request outlived the fade, none at all, so the spotlight simply popped on and off. It was the one effect with no visible entry or exit animation.

*The fix:* stop making the fade a round-trip. Ask for the overlay's **alpha-independent** appearance once and apply the ramp at draw time, where it costs nothing and runs at full frame rate.

### Returns

- `requestAlpha` - the `Spot.alpha` to ASK the backend for. `1` on the separable path, so the cached PNG is a reference image - which is what the cache key has always *claimed* it is, since `spotParamsKey` deliberately excludes alpha.
- `drawAlpha` - the `ctx.globalAlpha` to blit a SEPARABLE cached overlay at. `1` on the fallback path.
- `separable` - whether this frame's overlay may be faded client-side. The loop stores this **next to the cached `Image`**, not per tick: a response can land after the plan has moved on, and what matters at draw time is the basis the cached pixels were actually rendered at.

### Implementation

Separable requires BOTH conditions:

1. **The overlay is spotlight-only.** `overlayHasClicks` is true for a click-fx style `ripplePreview.ts` does not mirror (Pulse/Glow/Neon/Particles), whose rings are baked into this same PNG - fading the layer for the spotlight would fade those too. The video-fx wash cannot appear here at all (`fxOverlay.ts` never sends `videoAlpha`/`videoT`), so the click styles are the only mixing case. The gate is the **style**, not the live hit list, so it cannot flicker on and off mid-fade as individual clicks expire.
2. **The mode is in `ALPHA_LINEAR_SPOT_MODES`**, so the scaling is exact rather than merely plausible.

Otherwise it returns today's behaviour verbatim - request at the live alpha, blit at `1` - so every non-separable case stays pixel-identical to before this change. A `null` `resolved` (nothing lit) returns `drawAlpha: 0`, so a separable cached overlay reaches true zero on the frame it should instead of waiting out one more round-trip.

### Behaviors

- `splits a linear-mode, spotlight-only overlay into a reference request + a live blit alpha`
- `keeps the backend round-trip when the overlay also carries click rings`
- `keeps the backend round-trip for a mode whose alpha response is not proportional`
- `covers every linear mode` - and pins the set's membership.
- `draws a separable overlay at 0 once nothing is lit, without waiting for a round-trip`
- `clamps the blit alpha into 0..1`
- `is a full ramp across a fade, not the two or three steps a bucketed round-trip gave` - every frame of the ramp reuses the SAME cached image (`requestAlpha` never varies).

### Used by

`useCompositeLoop` - once per tick, immediately after its single `resolveSpotlight` call and BEFORE the overlay blit (which is why that call moved earlier in the tick).
