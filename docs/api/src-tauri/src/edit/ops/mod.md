# src-tauri/src/edit/ops/mod.rs

Submodule overviews for the `ops` group.

## api

Pure business logic for mutating and measuring an `EditDoc`; the single write point for all document mutations. Key items: `apply` (dispatches an `EditOp` variant to mutate `doc` in place), `metrics` (derives a read-only `Metrics` summary from the current doc state), `EditOp` enum (all editor operations: `AddZoom`, `AddZoomFull`, `UpdateZoom`, `RemoveZoom`, `SetTrim`, `AddCut`, `SetSpeed`, `SetLayoutSeg`, `AddEffect`, `UpdateEffect`, `RemoveEffect`), `Metrics` struct (`duration_ms`, `kept_ms`, `zoom_count`, `cut_count`). The effect ops are delegated to `effects::apply_effect`.

## effects

Effect-region edit ops (`add/update/remove_effect`), split out of `api.rs` so each file stays under the size limit. Key item: `apply_effect` - `api::apply` delegates the three effect-op variants here; `AddEffect` generates an `e`-prefixed id, `UpdateEffect` patches start/end, `RemoveEffect` drops by id.
