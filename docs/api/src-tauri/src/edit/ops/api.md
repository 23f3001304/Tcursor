# src-tauri/src/edit/ops/api.rs

Pure business logic for mutating an `EditDoc`. Every write that the editor or AI director performs flows through `apply`. No I/O occurs here - callers load the doc, call `apply`, and persist the result themselves. (The read-only summary the UI displays moved to `edit/ops/metrics.rs`.)

## EditOp

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case", tag = "op")]
pub enum EditOp {
    AddZoom { at_ms: u32, dur_ms: u32 },
    AddZoomFull { at_ms: u32, dur_ms: u32, scale: f32 },
    UpdateZoom {
        id: String,
        start_ms: Option<u32>,
        end_ms: Option<u32>,
        scale: Option<f32>,
        target: Option<ZoomTarget>,
        easing: Option<String>,
        zoom_in_ms: Option<u32>,
        zoom_out_ms: Option<u32>,
        layer: Option<u32>,
    },
    RemoveZoom { id: String },
    ClearZooms,
    SetZoomCamAction { id: String, action: Option<CamZoomAction> },
    SetTrim { in_ms: u32, out_ms: u32 },
    SetAspect { aspect: Aspect },
    AddCut { start_ms: u32, end_ms: u32 },
    SetSpeed { start_ms: u32, end_ms: u32, factor: f32 },
    AddLayoutSeg { at_ms: u32, dur_ms: u32, layout: String, transition_out_ms: Option<u32>, easing_out: Option<String> },
    UpdateLayoutSeg { id: String, start_ms: Option<u32>, end_ms: Option<u32>, layout: Option<String>, transition_ms: Option<u32>, easing: Option<String>, transition_out_ms: Option<u32>, easing_out: Option<String> },
    RemoveLayoutSeg { id: String },
    SetArrangement { id: String, screen: Option<Option<PanelPose>>, cam: Option<Option<PanelPose>> },
    ClearArrangement { id: String },
    AddEffect { kind: EffectKind, start_ms: u32, end_ms: u32 },
    UpdateEffect { id: String, start_ms: Option<u32>, end_ms: Option<u32>, fade_in_ms: Option<u32>, fade_out_ms: Option<u32>, mode: Option<String>, dim: Option<f32>, radius: Option<f32>, feather: Option<f32>, layer: Option<u32> },
    RemoveEffect { id: String },
    AddCameraMove { t_ms: u32, x: f32, y: f32, size: f32 },
    UpdateCameraMove { id: String, t_ms: Option<u32>, x: Option<f32>, y: Option<f32>, size: Option<f32>, easing: Option<String> },
    RemoveCameraMove { id: String },
}
```

Discriminated-union command type serialized to/from the Tauri IPC channel and the AI director. Each variant maps to one user-visible editing action.

- `AddZoom` - *insert a zoom at `at_ms` lasting `dur_ms` with default scale (2.0) and target (Cursor); the most common shortcut action from the editor timeline.*
- `AddZoomFull` - *same as `AddZoom` but the caller supplies `scale`; used by the AI director when it picks a specific zoom level from its plan.*
- `UpdateZoom` - *partial update by `id`; only `Some` fields are written, so the frontend can patch a single changed field without re-transmitting the full zoom.*
- `RemoveZoom` - *drop a zoom by string id; triggered by the delete key and AI-plan rollback.*
- `ClearZooms` - *drop every zoom in one step; the AI director's opening "rethink" move for the agentic auto-edit reveal - the frontend applies this first (when the doc already has zooms) so the reveal shows the mechanical seed-time auto-zooms clearing before the LLM's own picks land one at a time. Field-less, so it carries no payload beyond the `op` tag.*
- `SetZoomCamAction` - *set (`Some`) or clear (`None`) one zoom's webcam-on-zoom override. **Why a dedicated op rather than a field on `UpdateZoom`:** that op's "field is `None` => leave unchanged" convention cannot express "clear back to inherit the global default" without an `Option<Option<_>>`, which serializes ambiguously over IPC.*
- `SetTrim` - *replace the clip trim window atomically; in/out always travel together so no partial-update variant is needed.*
- `SetAspect` - *replace the output frame aspect ratio (`EditDoc.aspect`); `FrameRenderer::new` re-resolves `Layout` from it on the next build (export or preview).*
- `AddCut` - *append a cut segment; cut order and overlap resolution are rendering concerns, not enforced here.*
- `SetSpeed` - *append a speed segment with the given `factor`; the id is auto-assigned and the caller controls ordering via the plan.*
- `AddLayoutSeg` / `UpdateLayoutSeg` / `RemoveLayoutSeg` - *add/patch/remove a named-layout segment (`"screen"`, `"camera"`, `"presenter"`, ...), auto-id `l{n}`, clamped to the clip duration. Both carry the exit-transition pair as `Option`s: on `Add` they seed the segment (omitted = the `0`/hard-cut default, so every pre-existing caller is unchanged), on `Update` they follow the usual "only `Some` fields are written" rule. `easing_out` runs through `valid_easing` like every other easing setter.*
- `SetArrangement` / `ClearArrangement` - *set/hide a segment's panel poses, or drop the arrangement so the segment resolves from its `layout` preset again (T34); dispatched to `edit::ops::arrangement::apply_arrangement`, which documents the full semantics. Each panel field is THREE-valued on the wire - key absent = "leave this panel alone", `null` = hide it, an object = that pose - which needs `arrangement::double_option` to deserialize, because plain `Option<Option<_>>` folds `null` into the outer `None` and would make "hide" indistinguishable from "don't touch". (This is the same ambiguity `SetZoomCamAction` exists to avoid; here the three-valued shape is unavoidable because two independent panels are patched by one op, so it is handled explicitly instead.)*
- `AddEffect` / `UpdateEffect` / `RemoveEffect` - *add/patch/remove a Spotlight effect region; dispatched to `edit::ops::effects::apply_effect`.*
- `AddCameraMove` - *append a webcam PiP keyframe at `t_ms` with center `(x, y)` and `size`, default easing `"smooth"`; auto-id `k{n}` (max existing `k`-suffix + 1); `t_ms` clamped to `[0, dur_bound(doc)]`, `x`/`y`/`size` clamped to `[0.0, 1.0]`. `doc.camera_moves` is kept sorted by `t_ms` after every add.*
- `UpdateCameraMove` - *partial patch by `id`; only `Some` fields are written, same clamps as `AddCameraMove`; re-sorts `doc.camera_moves` by `t_ms` only when `t_ms` itself changed.*
- `RemoveCameraMove` - *drop a keyframe by id via `retain`; unknown ids are a no-op.*

### Used by

- `src-tauri/src/edit/commands.rs` - `apply_edit_op` Tauri command deserializes from IPC and forwards here
- `src-tauri/src/ai/commands.rs` - AI director dispatches a sequence of `EditOp`s from a generated plan

## apply

```rust
pub fn apply(doc: &mut EditDoc, op: EditOp) -> ()
```

Mutates `doc` in place by dispatching on `op`. The single write point for all `EditDoc` mutations.

### Inputs

- `doc: &mut EditDoc` - the document to mutate. *Why mutable ref rather than consume-and-return:* callers own the doc lifetime (loaded from disk, saved back) and must inspect it after the call.
- `op: EditOp` - the operation to apply. *Why an enum rather than individual functions:* a single call site enables IPC deserialization, AI plan replay, and future undo stacks without branching in every caller.

### Returns

`()`. Mutation is in place; the caller reads back `doc` and persists it.

### Implementation

1. **AddZoom / AddZoomFull** - generate id via `next_zoom_id` (finds the max numeric suffix among existing `z`-prefixed ids, increments by 1, falls back to `len`). Push `Zoom` with `target=Cursor`, `easing="smooth"`, and `scale=2.0` (`AddZoom`) or the caller-supplied scale (`AddZoomFull`). `saturating_add` guards the `end_ms` against u32 overflow. Both `start_ms`/`end_ms` clamp to `region::dur_bound(doc)` - the TRUE clip length (`doc.clip_ms`) when known, not the possibly-earlier `trim.out_ms`, so adding a region past where the clip is currently trimmed to no longer collapses it to the trim point.
2. **UpdateZoom** - linear scan by `id`; write only the `Some` fields into the found entry, with `start_ms`/`end_ms` clamped to the clip, then run through `region::clamp_order(&mut start_ms, &mut end_ms, start_ms_was_set)` (M5) so a partial update can never leave the zoom inverted - whichever handle this call actually set wins, the untouched partner is pulled to match it instead of persisting `start > end`. `easing` is passed through `valid_easing` (now in `edit::ops::region` - see `region.md`; an unrecognized name becomes `"smooth"`, a well-formed `cubic(x1,y1,x2,y2)` is kept in canonical form, matching `UpdateLayoutSeg` and `UpdateCameraMove` - it used to be the one easing setter that wrote the caller's string verbatim, so a typo'd or stale name reached the renderer). *Why linear scan:* zoom lists are short (typically fewer than 20 entries) so a map would cost more in bookkeeping than it saves in lookup.
3. **RemoveZoom** - single `retain` pass; no reindexing of remaining zooms.
4. **ClearZooms** - `doc.zooms.clear()`; drops every zoom in one call with no per-id lookup, unlike the single-target `RemoveZoom`.
5. **SetTrim** - full field replacement; `Trim` has two fields that are always logically coupled.
6. **AddCut** - push; no overlap check here since overlap rendering is a display concern.
7. **SetSpeed** - generate id via `next_speed_id` (same max-suffix strategy, prefix `s`), push `Speed`. The caller supplies ordering.
8. **UpdateLayoutSeg** - linear scan by `id`; writes only the `Some` fields (`start_ms`/`end_ms` clamped to the clip, `layout`/`easing`/`easing_out` validated same as above), then also runs `region::clamp_order` on `start_ms`/`end_ms` (M5), same as `UpdateZoom`.
9. **SetArrangement / ClearArrangement** - delegated whole to `edit::ops::arrangement::apply_arrangement`, the same shape the effect ops use.

### Behaviors

- `add_zoom_appends_with_correct_span` - `end_ms = at_ms + dur_ms`, defaults are `scale=2.0`, `easing="smooth"`.
- `add_zoom_yields_distinct_ids` - two consecutive adds produce different `z`-prefixed ids.
- `update_zoom_changes_only_supplied_fields` - supplying only `start_ms: Some(100)` leaves `end_ms` and `scale` unchanged.
- `update_zoom_unknown_id_is_noop` - an unknown id produces no panic and no mutation.
- `update_zoom_sets_layer_and_validates_easing` - `layer` and a known `easing` (`"spring"`) both apply; a subsequent unknown `easing` (`"bogus"`) is coerced to `"smooth"` rather than stored verbatim.
- `remove_zoom_drops_by_id` - only the targeted zoom is removed; others survive.
- `set_trim_replaces_trim` - both `in_ms` and `out_ms` update atomically.
- `set_aspect_replaces_aspect` - `doc.aspect` starts at `Source` and updates to the given variant.
- `add_zoom_full_uses_given_scale` - scale is preserved, not overridden to 2.0.
- `set_layout_seg_noop_unknown` - unknown segment id is silently ignored.
- `add_zoom_bounds_to_clip_ms_not_the_trim_point` (`api_tests.rs`) - `clip_ms=60_000` with an earlier `trim.out_ms=10_000`: a zoom added at `at_ms=30_000` is NOT collapsed to the trim point.
- `update_zoom_start_past_end_pulls_end_to_match` / `update_zoom_end_before_start_pulls_start_to_match` - M5: dragging one handle past the other pulls the untouched handle to meet it rather than persisting an inverted region.
- `update_layout_seg_start_past_end_pulls_end_to_match` - same M5 guarantee for `UpdateLayoutSeg`.

`Metrics` and `metrics` now live in `edit/ops/metrics.rs` (see `metrics.md`) - this file was at the size budget, and doc statistics are a separate responsibility from doc mutation.
