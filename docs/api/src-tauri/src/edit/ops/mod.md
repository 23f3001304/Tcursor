# src-tauri/src/edit/ops/mod.rs

Submodule overviews for the `ops` group.

## api

Pure business logic for mutating an `EditDoc`; the single write point for all document mutations. Key items: `apply` (dispatches an `EditOp` variant to mutate `doc` in place), `EditOp` enum (all editor operations: `AddZoom`, `AddZoomFull`, `UpdateZoom`, `RemoveZoom`, `SetTrim`, `AddCut`, `SetSpeed`, `AddLayoutSeg`/`UpdateLayoutSeg`/`RemoveLayoutSeg`, `SetArrangement`/`ClearArrangement`, `AddEffect`, `UpdateEffect`, `RemoveEffect`, the `*CameraMove` trio). The effect ops are delegated to `effects::apply_effect` and the arrangement ops to `arrangement::apply_arrangement`.

## arrangement

Arrangement edit ops (T34), split out of `api.rs`. Key items: `apply_arrangement` - `api::apply` delegates `SetArrangement`/`ClearArrangement` here; `clamp_pose` (bounds a `PanelPose` on the way in); `double_option` (the `deserialize_with` that keeps an absent panel key distinct from an explicit `null`). Enforces the at-least-one-visible-panel rule.

## metrics

Read-only summary statistics over an `EditDoc`, split out of `api.rs` (size budget) - the one thing in this group that never mutates the doc. Key items: `metrics(doc) -> Metrics`, `Metrics` struct (`duration_ms`, `kept_ms`, `zoom_count`, `cut_count`).

## effects

Effect-region edit ops (`add/update/remove_effect`), split out of `api.rs` so each file stays under the size limit. Key item: `apply_effect` - `api::apply` delegates the three effect-op variants here; `AddEffect` generates an `e`-prefixed id, `UpdateEffect` patches start/end, `RemoveEffect` drops by id.

## timeops

The cut and speed-span ops with their normalisation (`apply_time_op`, `normalize_cuts`, `normalize_speed`), kept apart from `api.rs`; `api::apply` tries them first.
