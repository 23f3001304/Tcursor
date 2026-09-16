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
        smart_typing: Option<bool>,
        #[serde(default)] easing_out: Option<String>,
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
    UpdateEffect { id: String, start_ms: Option<u32>, end_ms: Option<u32>, fade_in_ms: Option<u32>, fade_out_ms: Option<u32>, mode: Option<String>, dim: Option<f32>, radius: Option<f32>, feather: Option<f32>, layer: Option<u32>,
        #[serde(default)] rect: Option<[f32; 4]>, #[serde(default)] strength: Option<f32>, #[serde(default)] roundness: Option<f32> },
    RemoveEffect { id: String },
    AddCameraMove { t_ms: u32, x: f32, y: f32, size: f32,
        #[serde(default)] shape: Option<String>, #[serde(default)] roundness: Option<f32> },
    UpdateCameraMove { id: String, t_ms: Option<u32>, x: Option<f32>, y: Option<f32>, size: Option<f32>, easing: Option<String>,
        #[serde(default)] shape: Option<String>, #[serde(default)] roundness: Option<f32> },
    RemoveCameraMove { id: String },
    AddText { at_ms: u32, dur_ms: u32, #[serde(default)] kind: TextKind },
    UpdateText {
        id: String,
        start_ms: Option<u32>, end_ms: Option<u32>,
        text: Option<String>, sub: Option<Option<String>>,
        kind: Option<TextKind>, style: Option<String>,
        pos: Option<TextAnchor>, offset: Option<[f32; 2]>, size: Option<TextSize>,
        anim_in: Option<TextAnim>, anim_out: Option<TextAnim>,
        in_ms: Option<u32>, out_ms: Option<u32>, easing: Option<String>,
    },
    RemoveText { id: String },
    SplitAt { at_ms: u32 },
    MoveClip { id: String, to_index: usize },
    UpdateClip { id: String, src_in_ms: Option<u32>, src_out_ms: Option<u32>, transition_in_ms: Option<u32> },
    RemoveClip { id: String },
    ApplyMotionDefault,
}
```

Discriminated-union command type serialized to/from the Tauri IPC channel and the AI director. Each variant maps to one user-visible editing action.

- `AddZoom` - *insert a zoom at `at_ms` lasting `dur_ms` with default scale (2.0) and target (Cursor); the most common shortcut action from the editor timeline. Its curves come from `doc.settings.motion` (`ops::motion::for_zoom`) rather than the `"smooth"` this op used to hardcode - see the **Motion default** note below.*
- `AddZoomFull` - *same as `AddZoom` but the caller supplies `scale`; used by the AI director when it picks a specific zoom level from its plan.*
- `ApplyMotionDefault` - *stamp `doc.settings.motion` onto EVERY zoom, layout segment and camera move at once (Settings > Motion's "Apply to all regions"). Delegated to `ops::motion::apply_default`. A single op, so the whole sweep is one undo step - the same reason `AddCuts` exists as a batch rather than a loop of `AddCut`. Only the curves change: no duration, target, pose, layer or span is touched.*
- `UpdateZoom` - *partial update by `id`; only `Some` fields are written, so the frontend can patch a single changed field without re-transmitting the full zoom. `smart_typing: Some(true)` marks the zoom as smart-typing; `apply` only stores the flag, and `edit::commands::apply_edit_op` then refits the end from the recording's typing (`ops::smart_zoom`) - the pure op cannot, since it has no access to `typing.json`.*
- `RemoveZoom` - *drop a zoom by string id; triggered by the delete key and AI-plan rollback.*
- `ClearZooms` - *drop every zoom in one step; the AI director's opening "rethink" move for the agentic auto-edit reveal - the frontend applies this first (when the doc already has zooms) so the reveal shows the mechanical seed-time auto-zooms clearing before the LLM's own picks land one at a time. Field-less, so it carries no payload beyond the `op` tag.*
- `SetZoomCamAction` - *set (`Some`) or clear (`None`) one zoom's webcam-on-zoom override. **Why a dedicated op rather than a field on `UpdateZoom`:** that op's "field is `None` => leave unchanged" convention cannot express "clear back to inherit the global default" without an `Option<Option<_>>`, which serializes ambiguously over IPC.*
- `SetTrim` - *replace the clip trim window atomically; in/out always travel together so no partial-update variant is needed.*
- `SetAspect` - *replace the output frame aspect ratio (`EditDoc.aspect`); `FrameRenderer::new` re-resolves `Layout` from it on the next build (export or preview).*
- `AddCut` - *append a cut segment; cut order and overlap resolution are rendering concerns, not enforced here.*
- `SetSpeed` - *append a speed segment with the given `factor`; the id is auto-assigned and the caller controls ordering via the plan.*
- `AddLayoutSeg` / `UpdateLayoutSeg` / `RemoveLayoutSeg` - *add/patch/remove a named-layout segment (`"screen"`, `"camera"`, `"presenter"`, ...), auto-id `l{n}`, clamped to the clip duration. Both carry the exit-transition pair as `Option`s: on `Add` they seed the segment (omitted = `NEW_LAYOUT_TRANSITION_MS`, the same value the entry gets, so a new segment is symmetric - see that const), on `Update` they follow the usual "only `Some` fields are written" rule. `easing_out` runs through `valid_easing` like every other easing setter, and defaults to `"smooth"` - again the same as the entry.*
- `SetArrangement` / `ClearArrangement` - *set/hide a segment's panel poses, or drop the arrangement so the segment resolves from its `layout` preset again (T34); dispatched to `edit::ops::arrangement::apply_arrangement`, which documents the full semantics. Each panel field is THREE-valued on the wire - key absent = "leave this panel alone", `null` = hide it, an object = that pose - which needs `arrangement::double_option` to deserialize, because plain `Option<Option<_>>` folds `null` into the outer `None` and would make "hide" indistinguishable from "don't touch". (This is the same ambiguity `SetZoomCamAction` exists to avoid; here the three-valued shape is unavoidable because two independent panels are patched by one op, so it is handled explicitly instead.)*
- `AddEffect` / `UpdateEffect` / `RemoveEffect` - *add/patch/remove an effect region (Spotlight or a mask kind); dispatched to `edit::ops::effects::apply_effect`. `UpdateEffect` also carries a mask's `rect`, `strength` and `roundness` (all three `#[serde(default)]` - absent leaves the field unchanged); a mask ignores `mode`/`radius`, a Spotlight ignores `rect` - see `effects.md` for the clamps.*
- `AddCameraMove` - *set the webcam PiP keyframe at `t_ms`: center `(x, y)`, `size`, and optionally `shape` (`valid_cam_shape`, default `"layout"`) and `roundness` (clamped `[0.0, 0.5]`, default `DEFAULT_CAM_ROUNDNESS`); default easing `"smooth"`; auto-id `k{n}` (max existing `k`-suffix + 1); `t_ms` clamped to `[0, dur_bound(doc)]`, `x`/`y`/`size` clamped to `[0.0, 1.0]`. **One keyframe per instant:** if a keyframe already sits within `CAM_KF_SNAP_MS` (60 ms, the panel's own snap window - `region.md`) of the clamped `t_ms`, the nearest one is updated in place - its pose always, its `shape`/`roundness` only when the op carried them, its id/easing kept - rather than a duplicate pushed. The Move-mode slider commits with whatever doc it rendered with, which can be one IPC round-trip stale, so two quick commits both saying "add" used to stack a coincident duplicate that the sampler (which prefers the LAST coincident keyframe) then showed instead of the latest value; and an add that omitted the shape used to reset a circle keyframe to `"layout"` (bug sweep 2026-09-15). `doc.camera_moves` is kept sorted by `t_ms` after every add.*
- `UpdateCameraMove` - *partial patch by `id`; only `Some` fields are written, same clamps/validation as `AddCameraMove`; re-sorts `doc.camera_moves` by `t_ms` only when `t_ms` itself changed.*
- `RemoveCameraMove` - *drop a keyframe by id via `retain`; unknown ids are a no-op.*
- `AddText` / `UpdateText` / `RemoveText` - *add/patch/remove an animated text overlay (a title, lower third, stat or callout); dispatched to `edit::ops::textops::apply_text`, which documents the full validation (`textops.md`). `AddText` seeds `text`/`sub`/`size`/`pos`/`style`/`anim_in` from a fixed table keyed by `kind`; auto-id `t{n}` (max existing `t`-suffix + 1, `ids::next_text_id`), span clamped to `[0, dur_bound(doc)]`. `UpdateText`'s `sub` is double-optional on the wire the same way `SetArrangement`'s panels are (key absent = leave it, `null` = clear to `None`, a string = set it), deserialized through the same `arrangement::double_option` helper. Nothing renders `EditDoc.texts` yet - this batch only adds the ops.*
- `SplitAt` / `MoveClip` / `UpdateClip` / `RemoveClip` - *the clip track (spec 6.6): split the recording, reorder, retime a source range and its cross-dissolve, remove; dispatched to `edit::ops::clipops::apply_clip`, which documents the full semantics (`clipops.md`). `SplitAt` materialises `doc.clips` from the trim on its first call and auto-ids every new clip `cl{n}` (`ids::next_clip_id`); `RemoveClip` never takes the list below one clip. Every one of the four is a no-op on a doc whose length cannot yet be resolved (`region::dur_bound(doc) == u32::MAX`). Nothing renders `EditDoc.clips` yet - this batch only adds the ops (Batch 4 renders them).*
- `UpdateCaption` / `RemoveCaption` / `MergeCaptions` / `SplitCaption` / `SetCaptions` / `ClearCaptions` - *the caption track (M5), dispatched in one arm to `edit::ops::captions::apply_caption`, which documents the full semantics. `SetCaptions` is the one a human never sends: the ASR pass writes the whole track through it under the doc lock (plan ADDED-8), so a transcription is one undoable step and no caption array ever travels back over IPC. The other five are the timeline lane's and the Captions panel's: a drag commits `UpdateCaption`, which also carries the text edit, and Merge/Split are the two structural edits a caption line needs that no other region has.*

### Used by

- `src-tauri/src/edit/commands.rs` - `apply_edit_op` Tauri command deserializes from IPC and forwards here
- `src-tauri/src/ai/commands.rs` - AI director dispatches a sequence of `EditOp`s from a generated plan

## NEW_LAYOUT_TRANSITION_MS

```rust
pub const NEW_LAYOUT_TRANSITION_MS: u32 = 350;
```

The cross-fade duration a **newly created** layout segment gets, for BOTH its entry and its exit, so `AddLayoutSeg` produces a symmetric segment. Every creation path inherits it: the Effects panel's "Layout Segment" card (`EditorPanels.tsx`), the timeline drop handler (`Timeline.tsx`), and the AI director all go through this one op, none of which passes `transition_out_ms`.

*Why a named const and not two literals:* the exit used to be `transition_out_ms.unwrap_or(0)` while the entry was a hard-coded `350`, which is exactly the "layout effects animate in but hard-cut out" bug - a user-created segment faded in and then snapped. Deriving both from one const makes that asymmetry impossible to reintroduce by editing one site.

*Why this is NOT the serde default:* `LayoutSeg::transition_out_ms` keeps `#[serde(default)]` (i.e. `0`), and must forever. That default is the back-compat contract for docs written before exit transitions existed - a project saved then still renders its hard cuts byte-identically, because a *missing field* means "this doc predates the feature", not "give me today's default". Two different defaults for the same field on purpose, pinned by `add_layout_seg_default_does_not_leak_into_loading_an_old_doc`.

*Why `edit::seed`'s `layout_from_actions` still writes `0`:* those segments are derived from recorded `SetLayout` hotkeys and are gapless back-to-back, so every exit but the last would stand down anyway (`LayoutTrack::successor`'s `next_entry_wins`) - while the LAST one, which ends at the clip's end with no successor, would newly fade the whole frame back to the base `screen` layout over the recording's final 350ms. That is a rendering change to existing recordings for no gain, so the seed path is deliberately left alone.

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

1. **AddZoom / AddZoomFull** - both delegate to `ops::motion::add_zoom(doc, at_ms, dur_ms, scale)`, the body they share verbatim (moved there in M5 T4 for headroom; `motion.md`). It generates the id via `ids::next_zoom_id` (finds the max numeric suffix among existing `z`-prefixed ids, increments by 1, falls back to `len`). Push `Zoom` with `target=Cursor`, the project's curves from `ops::motion::for_zoom` (`easing="smooth"` and no `easing_out` on an untouched project, i.e. byte-identical to what this op wrote before M3), and `scale=2.0` (`AddZoom`) or the caller-supplied scale (`AddZoomFull`). `saturating_add` guards the `end_ms` against u32 overflow. Both `start_ms`/`end_ms` clamp to `region::dur_bound(doc)` - the TRUE clip length (`doc.clip_ms`) when known, not the possibly-earlier `trim.out_ms`, so adding a region past where the clip is currently trimmed to no longer collapses it to the trim point.
2. **UpdateZoom** - linear scan by `id`; write only the `Some` fields into the found entry, with `start_ms`/`end_ms` clamped to the clip, then run through `region::clamp_order(&mut start_ms, &mut end_ms, start_ms_was_set)` (M5) so a partial update can never leave the zoom inverted - whichever handle this call actually set wins, the untouched partner is pulled to match it instead of persisting `start > end`. `easing_out` (M3) goes through `ops::motion::set_zoom_easing_out`, which is where that field's THREE-valued wire meaning lives: absent leaves the zoom's out ramp alone, an EMPTY string clears it back to "the same curve as `easing`", any other string is canonicalised and stored - and a value equal to the zoom's own `easing` collapses back to unset rather than storing a duplicate. *Why a sentinel string rather than the `Option<Option<_>>` that `SetArrangement` uses:* an empty string is not a legal curve to begin with (`valid_easing("")` degrades to `"smooth"`), so it can never collide with something a client legitimately means, and the op stays one flat struct on the wire. Because of the collapse, `easing` is applied FIRST when a call carries both - that ordering in `apply` is load-bearing. `easing` is passed through `valid_easing` (now in `edit::ops::region` - see `region.md`; an unrecognized name becomes `"smooth"`, a well-formed `cubic(x1,y1,x2,y2)` is kept in canonical form, matching `UpdateLayoutSeg` and `UpdateCameraMove` - it used to be the one easing setter that wrote the caller's string verbatim, so a typo'd or stale name reached the renderer). *Why linear scan:* zoom lists are short (typically fewer than 20 entries) so a map would cost more in bookkeeping than it saves in lookup.
3. **RemoveZoom** - single `retain` pass; no reindexing of remaining zooms.
4. **ClearZooms** - `doc.zooms.clear()`; drops every zoom in one call with no per-id lookup, unlike the single-target `RemoveZoom`.
5. **SetTrim** - full field replacement; `Trim` has two fields that are always logically coupled.
6. **AddCut / AddCuts / UpdateCut / RemoveCut / SetSpeed / UpdateSpeed / RemoveSpeed** - delegated whole to `edit::ops::timeops::apply_time_op` before the match below (see `timeops.md`): fresh ids, and the doc normalised after every write (cuts merged, speed spans kept disjoint, factors clamped). `AddCuts` is one op so a Remove silences batch is one undo step.
7. *(moved)* `next_speed_id` and `next_cut_id` live in `timeops`.
8. **UpdateLayoutSeg** - linear scan by `id`; writes only the `Some` fields (`start_ms`/`end_ms` clamped to the clip, `layout`/`easing`/`easing_out` validated same as above), then also runs `region::clamp_order` on `start_ms`/`end_ms` (M5), same as `UpdateZoom`.
9. **SetArrangement / ClearArrangement** - delegated whole to `edit::ops::arrangement::apply_arrangement`, the same shape the effect ops use.
10. **ApplyMotionDefault** - delegated whole to `edit::ops::motion::apply_default` (see `motion.md`).
11. **AddText / UpdateText / RemoveText** - delegated whole to `edit::ops::textops::apply_text` (see `textops.md`), the same shape the effect and arrangement ops use.
12. **SplitAt / MoveClip / UpdateClip / RemoveClip** - delegated whole to `edit::ops::clipops::apply_clip` (see `clipops.md`), the same shape.

### The motion default

`AddZoom`, `AddZoomFull`, `AddLayoutSeg` and `AddCameraMove` all used to hardcode `easing: "smooth"`. Since M3 each reads `doc.settings.motion` through `ops::motion` instead - `for_zoom` (a pair, the out ramp `Some` only when it differs from the in ramp), `for_layout` (both, always stored), `for_camera` (the IN curve only, since a keyframe has no exit ramp of its own). `AddLayoutSeg`'s own explicit `easing_out` argument still WINS over the project default when the caller passes one; the default is the fallback, not an override. On a project that never touches Settings > Motion the default is Soft, which IS `"smooth"`, so every one of these ops produces exactly the region it produced before - pinned by `the_default_project_still_writes_todays_smooth_everywhere` (`motion_tests.rs`).

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
- `add_layout_seg_is_symmetric_by_default_entry_and_exit` (`api_layout_tests.rs`) - a new segment's `transition_out_ms`/`easing_out` equal its `transition_ms`/`easing`.
- `add_layout_seg_explicit_exit_still_wins_over_the_default` - passing `Some(0)`/`Some("linear")` seeds those values; the default only fills an omitted field.
- `add_layout_seg_default_does_not_leak_into_loading_an_old_doc` - a `LayoutSeg` JSON with no `transition_out_ms` key still deserializes to `0`, not `NEW_LAYOUT_TRANSITION_MS`.

`Metrics` and `metrics` now live in `edit/ops/metrics.rs` (see `metrics.md`) - this file was at the size budget, and doc statistics are a separate responsibility from doc mutation.
