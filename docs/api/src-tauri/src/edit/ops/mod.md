# src-tauri/src/edit/ops/mod.rs

Submodule overviews for the `ops` group.

## api

Pure business logic for mutating an `EditDoc`; the single write point for all document mutations. Key items: `apply` (dispatches an `EditOp` variant to mutate `doc` in place), `EditOp` enum (all editor operations: `AddZoom`, `AddZoomFull`, `UpdateZoom`, `RemoveZoom`, `SetTrim`, `AddCut`, `SetSpeed`, `AddLayoutSeg`/`UpdateLayoutSeg`/`RemoveLayoutSeg`, `SetArrangement`/`ClearArrangement`, `AddEffect`, `UpdateEffect`, `RemoveEffect`, the `*CameraMove` trio). The effect ops are delegated to `effects::apply_effect` and the arrangement ops to `arrangement::apply_arrangement`.

## arrangement

Arrangement edit ops (T34), split out of `api.rs`. Key items: `apply_arrangement` - `api::apply` delegates `SetArrangement`/`ClearArrangement` here; `clamp_pose` (bounds a `PanelPose` on the way in); `double_option` (the `deserialize_with` that keeps an absent panel key distinct from an explicit `null`). Enforces the at-least-one-visible-panel rule.

## captions

```rust
pub mod captions;
```

The caption track's edit ops (M5 T4) - re-time, retype, remove, merge, split, replace, clear. `api::apply` routes all six caption variants here in one arm.

## metrics

Read-only summary statistics over an `EditDoc`, split out of `api.rs` (size budget) - the one thing in this group that never mutates the doc. Key items: `metrics(doc) -> Metrics`, `Metrics` struct (`duration_ms`, `kept_ms`, `zoom_count`, `cut_count`).

## effects

Effect-region edit ops (`add/update/remove_effect`), split out of `api.rs` so each file stays under the size limit. Key item: `apply_effect` - `api::apply` delegates the three effect-op variants here; `AddEffect` generates an `e`-prefixed id, `UpdateEffect` patches start/end, `RemoveEffect` drops by id.

## motion

The project's motion language applied to regions (M3), split out of `api.rs` the way `effects.rs` and `timeops.rs` are. Key items: `for_zoom` / `for_layout` / `for_camera` (what a NEWLY added region inherits from `doc.settings.motion`, replacing the `"smooth"` those add ops used to hardcode - a zoom's out ramp is returned as `Option` and stored only when it differs from its in ramp), `set_zoom_easing_out` (`UpdateZoom.easing_out`'s write: an EMPTY string clears back to "same as `easing`", and a value equal to `easing` collapses to unset), `apply_default` (`EditOp::ApplyMotionDefault` - stamp the project curve onto every zoom, layout segment and camera move as one undo step; curves only, never timing).

## smart_zoom

Smart typing duration for a manual zoom (owner, 2026-09-14). Key items: `smart_end` (pure: where a zoom starting at `start_ms` ends, given keystroke times on the same clock - a chain of keys each within `hold_ms` of the previous, the end one hold after the last), `typing_on_doc_clock` (`typing.json` shifted like the seed shifts every region, then through the doc's cuts and speed spans via `TimeMap::out_of`), `refit` (rewrites a smart zoom's `end_ms`; called by `edit::commands::apply_edit_op` after an `UpdateZoom` that switched the flag on or moved the start).

## timeops

The cut and speed-span ops with their normalisation (`apply_time_op`, `normalize_cuts`, `normalize_speed`), kept apart from `api.rs`; `api::apply` tries them first.
