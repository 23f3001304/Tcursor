# src-tauri/src/edit/ops/effects.rs

Effect-region edit ops (add/update/remove), split out of `api.rs` so each file stays focused. v1 handles Spotlight regions; `api::apply` delegates the three effect-op variants here.

## apply_effect

```rust
pub fn apply_effect(doc: &mut EditDoc, op: EditOp)
```

Applies an effect-region op:

- `AddEffect { kind, start_ms, end_ms }` pushes a new `EffectRegion` with a generated id (`next_effect_id`: `e0`, `e1`, ... mirroring the zoom ids).
- `UpdateEffect { id, start_ms?, end_ms? }` patches the supplied fields of the matching region.
- `RemoveEffect { id }` drops the region by id.

Any other op is a no-op (`_ => {}`) - `api::apply`'s match only routes the three effect variants here, so the fallback is unreachable by contract.

## lift_always_on_spotlight

```rust
pub fn lift_always_on_spotlight(doc: &mut EditDoc) -> bool
```

Converts an always-on `clickfx.spotlight` toggle into one full-span (`[0, trim.out_ms]`) editable Spotlight `EffectRegion` and sets the toggle off, so the region becomes the single source the editor can trim or remove. Returns whether it changed the doc. No-op when the toggle is already off, a Spotlight region already exists (so it is idempotent), or `trim.out_ms == 0` (a degenerate no-event-log doc, where a `[0,0]` region would be un-grabbable and would silently disable the spotlight).

**Why:** the always-on spotlight is a global setting the export renders continuously - not a timeline region - so it couldn't be edited or removed. `seed::load_or_seed` calls this on BOTH freshly-seeded and previously-saved docs (a lightweight migration), so an always-on spotlight becomes an editable pill without re-recording. Both preview and export honor it because they read `doc.settings.clickfx` + `doc.effects`, and `fx_state` takes `max(toggle, spotlight_region_alpha)` - with the toggle now off, the region drives the result (a full-span region fades in/out over `FADE_MS` at the very clip edges, unlike the old constant toggle).
