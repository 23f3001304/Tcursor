# src-tauri/src/export/fx/hold.rs

Generic fade-in / hold / fade-out alpha ramp shared by every hold-to-activate manual effect (spotlight, video FX). Accepts predicate closures identifying the start and end `ActionKind` so the same ramp algorithm drives all effect types without duplication. Pure and deterministic - identical inputs always produce identical output.

## hold_alpha

```rust
pub fn hold_alpha(
    actions: &[ActionEvent], et: u32, fade_ms: u32,
    is_start: impl Fn(ActionKind) -> bool, is_end: impl Fn(ActionKind) -> bool,
) -> f32
```

Computes the effect strength 0..1 at elapsed time `et` by walking the action event log and accumulating the maximum alpha across all start..end pairs.

### Inputs

- `actions: &[ActionEvent]` - the action event log in time order. *Why linear scan rather than binary search:* the log is small (user key presses) and the function is called once per frame; O(n) is negligible, and a linear scan naturally handles multiple overlapping pairs.
- `et: u32` - current frame event-time in milliseconds. *Why:* determines position within each start..end pair; the function must be stateless so the exporter can call it for any frame in any order.
- `fade_ms: u32` - duration of both the ramp-up and ramp-down in milliseconds; clamped internally to minimum 1 to avoid division-by-zero. *Why the same duration for both transitions:* symmetric fades are the design default; asymmetric fades would require two parameters and add complexity for a rare use case.
- `is_start: impl Fn(ActionKind) -> bool` - predicate matching the start event kind. *Why a closure, not an enum variant:* keeps this function generic so callers bind the specific `ActionKind` they care about without an extra match layer here.
- `is_end: impl Fn(ActionKind) -> bool` - predicate matching the end event kind. *Why paired closure:* start and end are always different action kinds; explicit predicates prevent matching the wrong pair.

### Returns

`f32` in `[0.0, 1.0]`. 0.0 when no start event has been seen or `et` is before every start. 1.0 during a fully-ramped hold. The maximum over all pairs when multiple pairs have been triggered.

### Implementation

1. Set `fade = max(fade_ms, 1) as f32` and `alpha = 0.0`. Track `start: Option<u32>` for the currently-open pair.
2. Walk `actions` in order:
   - On `is_start`: record `start = Some(a.t)`.
   - On `is_end` with a pending start `s`:
     - Compute `up = ((et - s) / fade).min(1.0)` - how far through the ramp-in we are.
     - Compute `down = if et >= a.t { (1.0 - (et - a.t) / fade).max(0.0) } else { 1.0 }` - how far through the ramp-out we are.
     - `alpha = alpha.max(up.min(down))`. *Why `up.min(down)`:* during the ramp-in phase `down = 1.0` so only ramp-in matters; during the hold both are 1.0; during ramp-out `up = 1.0` so only ramp-out matters; before start or after end the product collapses to 0.
     - Clear `start`.
3. After the loop, if `start` is still `Some(s)` and `et >= s` (unpaired open start): `alpha = alpha.max(((et - s) / fade).min(1.0))`. *Why:* an active hold with no end event keeps the effect at full strength; this is the normal state during a live recording.
4. Return `alpha`.

### Used by

- `src-tauri/src/export/fx/spot/spotlight.rs` - `spotlight::hold_alpha` delegates here with `SpotlightHoldStart` / `SpotlightHoldEnd` predicates.
- `src-tauri/src/export/fx/fx_state.rs` - directly calls `hold::hold_alpha` with `VideoFxHoldStart` / `VideoFxHoldEnd` predicates for the video FX overlay.

### Behaviors

- `ramps_up_holds_then_down` - with a `SpotlightHoldStart` at t=1000 and `SpotlightHoldEnd` at t=2000, `fade_ms=200`: alpha=0 at t=900; ~0.5 at t=1100; 1.0 at t=1500; ~0.5 at t=2100; 0 at t=2300.

## hold_spans

```rust
pub fn hold_spans(
    actions: &[ActionEvent], end_default: u32,
    is_start: impl Fn(ActionKind) -> bool, is_end: impl Fn(ActionKind) -> bool,
) -> Vec<(u32, u32)>
```

The held intervals as `(start, end)` event-time pairs (ms), rather than a single alpha. Same start/end pairing as `hold_alpha` (an unpaired still-held start runs to `end_default`), but it returns the raw spans so a consumer can apply its own ramp. Used by the editor-preview command `spotlight_holds` (`preview_track.rs`) to surface recorded holds the way `click_track` surfaces clicks - the preview applies the fade ramp itself, in TypeScript, to match `hold_alpha`.

### Inputs

- `actions: &[ActionEvent]` - the action event log in time order.
- `end_default: u32` - the end time used for an unpaired (still-open) start. *Why:* a hold with no recorded end runs to the timeline end; the caller passes the event-time that maps to the output end.
- `is_start` / `is_end` - the same start/end predicates as `hold_alpha`, so one helper serves spotlight and video-FX holds.

### Returns

`Vec<(u32, u32)>` - one `(start, end)` pair per hold, in event time, in event order.

### Behaviors

- `spans_pair_and_unpaired_runs_to_default` - starts at 1000/3000 with one end at 2000 and `end_default=5000` yield `[(1000, 2000), (3000, 5000)]`.
