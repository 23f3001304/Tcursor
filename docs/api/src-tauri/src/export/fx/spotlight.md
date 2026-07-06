# src-tauri/src/export/fx/spotlight.rs

Thin spotlight-specific adapter over the generic `hold::hold_alpha` ramp. Provides a typed public function for the spotlight effect so callers in `fx_state` do not need to supply predicate closures inline. All logic lives in `export::hold`; this file only binds the correct `ActionKind` predicates.

## hold_alpha

```rust
pub fn hold_alpha(actions: &[ActionEvent], et: u32, fade_ms: u32) -> f32
```

Returns the spotlight strength 0..1 at event-time `et`, ramping in and out around `SpotlightHoldStart` / `SpotlightHoldEnd` pairs.

### Inputs

- `actions: &[ActionEvent]` - the recorded action event log. *Why:* scanned by `hold::hold_alpha` to find `SpotlightHoldStart`/`End` pairs relative to `et`.
- `et: u32` - current frame event-time in milliseconds. *Why:* determines position within each start..end pair's ramp-in, hold, and ramp-out phases.
- `fade_ms: u32` - duration of both the ramp-up and ramp-down transitions in milliseconds. *Why caller-supplied:* `fx_state` passes a shared `FADE_MS` constant that can be tuned once for all hold-to-activate effects.

### Returns

`f32` in `[0.0, 1.0]`. Delegates to `hold::hold_alpha` with predicates `is_start = SpotlightHoldStart` and `is_end = SpotlightHoldEnd`. Behaviour mirrors `hold::hold_alpha` exactly:
- Before a `SpotlightHoldStart`: 0.0.
- During ramp-up `[start, start + fade_ms]`: linear 0 to 1.
- During hold `[start + fade_ms, end]`: 1.0.
- During ramp-down `[end, end + fade_ms]`: linear 1 to 0.
- After ramp-down: 0.0.
- Unpaired `SpotlightHoldStart` (no matching end): ramps to 1.0 and stays there.
- Multiple overlapping pairs: returns the maximum alpha.

### Used by

- `src-tauri/src/export/fx/fx_state.rs` - `FxState::build` calls `spotlight::hold_alpha` to compute `s_alpha`; the result drives whether a `Spot` is included in the `FxState` snapshot for the current frame.

### Behaviors

- `ramps_up_holds_then_ramps_down` - at t=900 (before start=1000): 0.0; at t=1100 (100ms into 200ms ramp-up): ~0.5; at t=1500 (held): 1.0; at t=2100 (100ms into ramp-down from end=2000): ~0.5; at t=2300 (after ramp-down): 0.0.
- `unpaired_start_stays_on` - a `SpotlightHoldStart` with no matching end returns 1.0 at any time after the ramp-in completes.
