# src-tauri/src/edit/ops/effects.rs

Effect-region edit ops (add/update/remove), split out of `api.rs` so each file stays focused. It covers all four `EffectKind` variants - Spotlight, and the three masks Blur, Pixelate and Highlight - and carries the mask validation with them: `clamp_rect` holds a mask's canvas-fraction rect inside the canvas with a minimum size, `strength` is clamped to `STRENGTH_RANGE` and `roundness` to `0..0.5`, `add_effect` seeds a mask with `MASK_SEED_RECT` while a spotlight gets no rect at all, and a spotlight ignores rect/strength/roundness exactly as a mask ignores mode and radius. `api::apply` delegates the three effect-op variants here.

## apply_effect

```rust
pub fn apply_effect(doc: &mut EditDoc, op: EditOp)
```

Applies an effect-region op:

- `AddEffect { kind, start_ms, end_ms }` pushes a new `EffectRegion` with a generated id (`next_effect_id`: `e0`, `e1`, ... mirroring the zoom ids); a new mask is seeded with `MASK_SEED_RECT`.
- `UpdateEffect { id, start_ms?, end_ms? }` patches the supplied fields of the matching region, then runs `region::clamp_order` on the result (M5) so a partial update can never leave `start_ms > end_ms` persisted.
- `rect`: every component finite; `w` and `h` clamped to `[0.01, 1.0]`; `x` to `[0.0, 1.0 - w]` and `y` to `[0.0, 1.0 - h]`, so a mask can never leave the canvas. A non-finite rect is dropped and the stored one kept.
- `strength`: clamped to `[0.002, 0.120]`. Below the floor a blur does nothing and a pixel cell is one pixel; above the ceiling the blur costs more than the frame.
- `roundness`: clamped to `[0.0, 0.5]`, the range `CameraMove::roundness` uses.
- `rect` is ignored on a Spotlight, and `mode`/`radius` on a mask, so a stale field can never change a look.
- `RemoveEffect { id }` drops the region by id.

Any other op is a no-op (`_ => {}`) - `api::apply`'s match only routes the three effect variants here, so the fallback is unreachable by contract.

### Behaviors

- `adding_a_mask_seeds_the_centred_rect_and_a_spotlight_gets_none` - `AddEffect` with a mask kind seeds `rect` to `MASK_SEED_RECT`; a Spotlight gets `rect: None`.
- `a_rect_is_clamped_so_it_never_leaves_the_canvas` - `clamp_rect` clamps an oversized/offset rect into `[0, 1]` and returns `None` for a non-finite component; an `UpdateEffect` carrying a non-finite `rect` leaves the region's stored rect unchanged.
- `strength_and_roundness_are_clamped_to_their_ranges` - values outside `[0.002, 0.120]` and `[0.0, 0.5]` are clamped, not rejected or dropped.
- `a_rect_is_ignored_on_a_spotlight_and_mode_and_radius_on_a_mask` - a Spotlight's `rect` write is a no-op; a mask's `mode`/`radius` writes are no-ops while its `dim`/`feather` writes still apply.
- `update_effect_parses_with_the_mask_fields_absent` - an `update_effect` JSON op with no `rect`/`strength`/`roundness` keys deserializes with all three `None` (`#[serde(default)]`).

## MASK_SEED_RECT

```rust
pub const MASK_SEED_RECT: [f32; 4] = [0.35, 0.40, 0.30, 0.20];
```

The centred canvas-fraction rect (`[x, y, w, h]`) a new mask region's `rect` is seeded with on `AddEffect`, so it lands visible and draggable instead of needing to be placed blind.

## clamp_rect

```rust
pub fn clamp_rect(r: [f32; 4]) -> Option<[f32; 4]>
```

Clamps a canvas-fraction rect `[x, y, w, h]` so it never leaves the canvas: `w`/`h` to `[0.01, 1.0]`, then `x`/`y` to `[0.0, 1.0 - w]`/`[0.0, 1.0 - h]`. Returns `None` when any component is non-finite (`NaN`/`inf`), so the caller (`UpdateEffect`'s handling in `apply_effect`) keeps the region's existing stored rect rather than writing a broken one. Reused by Batch 2a's stage drag.

## lift_always_on_spotlight

```rust
pub fn lift_always_on_spotlight(doc: &mut EditDoc) -> bool
```

Converts an always-on `clickfx.spotlight` toggle into one full-span (`[0, region::dur_bound(doc)]`) editable Spotlight `EffectRegion` and sets the toggle off, so the region becomes the single source the editor can trim or remove. Returns whether it changed the doc. No-op when the toggle is already off, a Spotlight region already exists (so it is idempotent), or `dur_bound(doc) == u32::MAX` (a truly unseeded doc - neither `clip_ms` nor `trim.out_ms` known yet - where a `[0, u32::MAX]` region would be meaningless).

**Why `dur_bound`, not `trim.out_ms` (H2, bug-sweep-2):** the span used to be `[0, trim.out_ms]` with a `trim.out_ms == 0` guard treated as "degenerate no-event-log doc". Both were wrong once a doc has a real `clip_ms`: (1) `trim.out_ms` is the CURRENT trim point, not the clip length - lifting the toggle after trimming the tail bounded the region to the smaller trim, so widening the trim back out later left the spotlight stuck at the old boundary; (2) `trim.out_ms == 0` is also the ORDINARY "no trim / whole clip" sentinel (`Trim::resolve`'s contract) - a completely untrimmed doc hit the same guard and skipped the lift entirely, leaving the toggle permanently on with no editable region, which `SpotlightSim::resolve` then renders as a full-strength spotlight over the ENTIRE clip with no timeline pill to trim or delete (this is the bug the UX audit caught: a fresh recording playing entirely dimmed by default). `dur_bound` is the same clip_ms-first fallback chain every other region-placing op already uses, so a doc with a known `clip_ms` lifts correctly regardless of its current trim state.

**Why the toggle itself defaults off (R1):** `ClickFxSettings::default().spotlight == false` (`settings/model.rs`) - unchanged by this fix, verified still correct and pinned by `settings::model_tests::defaults_match_tuned_zoom_and_round_trip`. A persisted `config.json` where a user explicitly turned it on keeps that choice; only the record-TIME default is OFF.

**Why the lift exists at all:** the always-on spotlight is a global setting the export renders continuously - not a timeline region - so it couldn't be edited or removed. `seed::load_or_seed` calls this on BOTH freshly-seeded and previously-saved docs (a lightweight migration), so an always-on spotlight becomes an editable pill without re-recording. Both preview and export honor it because they read `doc.settings.clickfx` + `doc.effects`, and `fx_state` takes `max(toggle, spotlight_region_alpha)` - with the toggle now off, the region drives the result (a full-span region fades in/out over `FADE_MS` at the very clip edges, unlike the old constant toggle).

### Behaviors

- `lifts_always_on_spotlight_to_full_span_region_and_disables_toggle` - toggle on, `trim.out_ms=8000` (no `clip_ms`) lifts to `[0, 8000]` and flips the toggle off; idempotent on a second call.
- `lift_is_noop_when_spotlight_toggle_off` - toggle already off is a no-op.
- `lift_is_noop_on_a_truly_unseeded_doc` - `clip_ms == 0 && trim.out_ms == 0` (`dur_bound == u32::MAX`) skips the lift and leaves the toggle on.
- `lift_bounds_by_clip_ms_not_a_smaller_trim_out_ms` - `clip_ms=60_000`, `trim.out_ms=5_000` (a trimmed clip) lifts to `[0, 60_000]`, not `[0, 5_000]`.
- `lift_bounds_by_clip_ms_when_trim_out_ms_is_the_no_trim_sentinel` - `clip_ms=38_000`, `trim.out_ms=0` (ordinary "no trim" state) still lifts, to `[0, 38_000]`.
- `lift_falls_back_to_trim_out_ms_when_clip_ms_is_unknown` - `clip_ms=0`, `trim.out_ms=12_000` (a doc predating `clip_ms`) lifts to `[0, 12_000]`, matching `dur_bound`'s fallback chain.
