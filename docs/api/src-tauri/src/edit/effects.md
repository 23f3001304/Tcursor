# src-tauri/src/edit/effects.rs

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
