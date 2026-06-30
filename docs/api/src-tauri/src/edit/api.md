# src-tauri/src/edit/api.rs

Pure business logic for mutating and measuring an `EditDoc`. Every write that the editor or AI director performs flows through `apply`; `metrics` derives the read-only summary the UI displays. No I/O occurs here - callers load the doc, call these functions, and persist the result themselves.

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
    },
    RemoveZoom { id: String },
    SetTrim { in_ms: u32, out_ms: u32 },
    AddCut { start_ms: u32, end_ms: u32 },
    SetSpeed { start_ms: u32, end_ms: u32, factor: f32 },
    SetLayoutSeg { id: String, layout: String },
}
```

Discriminated-union command type serialized to/from the Tauri IPC channel and the AI director. Each variant maps to one user-visible editing action.

- `AddZoom` - *insert a zoom at `at_ms` lasting `dur_ms` with default scale (2.0) and target (Cursor); the most common shortcut action from the editor timeline.*
- `AddZoomFull` - *same as `AddZoom` but the caller supplies `scale`; used by the AI director when it picks a specific zoom level from its plan.*
- `UpdateZoom` - *partial update by `id`; only `Some` fields are written, so the frontend can patch a single changed field without re-transmitting the full zoom.*
- `RemoveZoom` - *drop a zoom by string id; triggered by the delete key and AI-plan rollback.*
- `SetTrim` - *replace the clip trim window atomically; in/out always travel together so no partial-update variant is needed.*
- `AddCut` - *append a cut segment; cut order and overlap resolution are rendering concerns, not enforced here.*
- `SetSpeed` - *append a speed segment with the given `factor`; the id is auto-assigned and the caller controls ordering via the plan.*
- `SetLayoutSeg` - *update the `layout` string on an existing segment matched by `id`; the segment list is seeded once in `seed.rs` and its structure never changes here.*

### Used by

- `src-tauri/src/edit/commands.rs` - `apply_edit_op` Tauri command deserializes from IPC and forwards here
- `src-tauri/src/ai/commands.rs` - AI director dispatches a sequence of `EditOp`s from a generated plan

## Metrics

```rust
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Metrics {
    pub duration_ms: u32,
    pub kept_ms: u32,
    pub zoom_count: usize,
    pub cut_count: usize,
}
```

Read-only doc summary returned to the frontend after every edit. Derived on demand from `EditDoc`; never stored.

- `duration_ms` - *raw clip length (`trim.out_ms`); sets the timeline ruler's right edge.*
- `kept_ms` - *`(trim.out - trim.in) - sum(cut durations within trim)`; drives the export file-size estimate shown in the sidebar.*
- `zoom_count` - *`doc.zooms.len()`; the editor badge indicating how many zoom events are active.*
- `cut_count` - *`doc.cuts.len()`; pairs with `kept_ms` so the user can confirm cuts were registered.*

### Used by

- `src-tauri/src/edit/commands.rs` - returned alongside the updated `EditDoc` in `apply_edit_op`

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

1. **AddZoom / AddZoomFull** - generate id via `next_zoom_id` (finds the max numeric suffix among existing `z`-prefixed ids, increments by 1, falls back to `len`). Push `Zoom` with `target=Cursor`, `easing="smooth"`, and `scale=2.0` (`AddZoom`) or the caller-supplied scale (`AddZoomFull`). `saturating_add` guards the `end_ms` against u32 overflow.
2. **UpdateZoom** - linear scan by `id`; write only the `Some` fields into the found entry. *Why linear scan:* zoom lists are short (typically fewer than 20 entries) so a map would cost more in bookkeeping than it saves in lookup.
3. **RemoveZoom** - single `retain` pass; no reindexing of remaining zooms.
4. **SetTrim** - full field replacement; `Trim` has two fields that are always logically coupled.
5. **AddCut** - push; no overlap check here since overlap rendering is a display concern.
6. **SetSpeed** - generate id via `next_speed_id` (same max-suffix strategy, prefix `s`), push `Speed`. The caller supplies ordering.
7. **SetLayoutSeg** - linear scan by `id`; mutates only the `layout` string. Layout segment structure is fixed by the seed.

### Behaviors

- `add_zoom_appends_with_correct_span` - `end_ms = at_ms + dur_ms`, defaults are `scale=2.0`, `easing="smooth"`.
- `add_zoom_yields_distinct_ids` - two consecutive adds produce different `z`-prefixed ids.
- `update_zoom_changes_only_supplied_fields` - supplying only `start_ms: Some(100)` leaves `end_ms` and `scale` unchanged.
- `update_zoom_unknown_id_is_noop` - an unknown id produces no panic and no mutation.
- `remove_zoom_drops_by_id` - only the targeted zoom is removed; others survive.
- `set_trim_replaces_trim` - both `in_ms` and `out_ms` update atomically.
- `add_zoom_full_uses_given_scale` - scale is preserved, not overridden to 2.0.
- `set_layout_seg_noop_unknown` - unknown segment id is silently ignored.

## metrics

```rust
pub fn metrics(doc: &EditDoc) -> Metrics
```

Derives a `Metrics` snapshot from the current state of `doc` without mutating it.

### Inputs

- `doc: &EditDoc` - the document to measure. *Why immutable ref:* `metrics` is a pure read; it must never be callable in a context that expects mutation.*

### Returns

`Metrics` with all four fields computed in a single pass.

### Implementation

1. `duration_ms = doc.trim.out_ms`.
2. `trim_span = trim.out_ms - trim.in_ms` (saturating).
3. For each cut, clamp it to `[trim_in, trim_out]`, compute clamped length, accumulate. *Why clamp:* cuts outside the trim window do not reduce the exported clip.
4. `kept_ms = trim_span - cut_sum` (saturating).
5. `zoom_count = doc.zooms.len()`, `cut_count = doc.cuts.len()`.

### Behaviors

- `metrics_kept_ms_subtracts_cuts` - a 10 s clip (`SetTrim {0, 10000}`) with a 2 s cut (`AddCut {1000, 3000}`) yields `kept_ms=8000`, `cut_count=1`.
