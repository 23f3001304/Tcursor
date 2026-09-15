# src-tauri/src/edit/model.rs

Data model for `edit.json`: the complete type hierarchy from atomic clip edits (`Trim`, `Cut`, `Zoom`, `Speed`, `LayoutSeg`) up to the root `EditDoc`, plus save/load helpers that make `EditDoc` self-serializing. All types derive `Serialize`/`Deserialize` so they transit IPC and disk without a separate DTO layer.

## DOC_VERSION

```rust
pub const DOC_VERSION: u32 = 2;
```

Current `edit.json` schema version - the value every doc this build writes carries, and the target `seed::migrate` upgrades older docs to.

- **v1** - `zooms` and `camera_moves` on the output clock, but `effects` and `layout` seeded straight off the recorded action log (event clock), so on a real recording they sat ~800 ms early against the editor timeline and the export disagreed with the preview.
- **v2** - the one-clock contract: EVERY region list in the doc is output time (0 = first video frame). `seed::v1_to_v2` shifts a v1 doc's `effects`/`layout` by `events_ms - video_start` on load.

Bump this constant and add one `if doc.version < N` step in `seed::migrate` for a future schema change.

### Used by

- `src-tauri/src/edit/model.rs` - `EditDoc::default`
- `src-tauri/src/edit/seed.rs` - `build_default` stamps it; `migrate` compares against it

## Trim

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct Trim { pub in_ms: u32, pub out_ms: u32 }
impl Default for Trim { fn default() -> Self { Self { in_ms: 0, out_ms: 0 } } }
```

The clip's in/out points in milliseconds, measured from the raw recording start.

- `in_ms` - *start of the exported region; frames before this are discarded. Default 0 = no head trim.*
- `out_ms` - *end of the exported region; frames after this are discarded. Also used as `duration_ms` in `metrics`. Default 0 signals "not yet set" and is replaced by `build_default` with the real clip duration.*

### Used by

- `src-tauri/src/edit/ops/api.rs` - `SetTrim` variant replaces both fields; `metrics` reads `in_ms`/`out_ms`
- `src-tauri/src/export/render/fromedit.rs` - clip bounds supplied to the compositor
- `src-tauri/src/export/pipeline/exporter.rs` - `export()` calls `Trim::resolve` to gate the frame loop
- `src-tauri/src/export/preview/preview_track.rs` - the frontend's playhead clamp reads the same resolved range via `resolveTrim` (`src/shared/edit.ts`)

## Trim::resolve

```rust
pub fn resolve(&self, total_dur_ms: u32) -> (u32, u32)
```

The effective `[in_ms, out_ms)` export/preview range against a clip of `total_dur_ms`. `out_ms == 0` (the doc-level default, "not yet set") means "no trim / whole clip"; both bounds are clamped into `[0, total_dur_ms]` and `in_ms` never exceeds the resolved `out_ms`, so a degenerate/inverted range safely collapses to zero-length instead of underflowing at the call site. The ONE function export (`exporter::export`) and preview both read the trim through, so they always agree on the effective range - mirrored on the TS side by `resolveTrim` (`src/shared/edit.ts`).

### Behaviors

- `default_trim_resolves_to_the_whole_clip` - `Trim::default()` (`{0,0}`) resolves to `(0, total_dur_ms)` - back-compat guard.
- `trim_resolve_clamps_in_to_out_and_both_to_the_clip` - an out beyond the real duration clamps down; an in beyond the resolved out clamps to it.

## Cut

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cut { #[serde(default)] pub id: String, pub start_ms: u32, pub end_ms: u32 }
```

A single removed time range of clip time. Rendered by the time remap (`export::remap::TimeMap`), which clamps it into the trim window, merges overlapping cuts and removes exactly the frames whose time lies inside it.

- `id` - *`c{n}`; defaulted to empty for docs saved before cuts had ids and filled on load by `EditDoc::assign_missing_ids`.*
- `start_ms` - *inclusive start of the removed span.*
- `end_ms` - *exclusive end of the removed span.*

### Used by

- `src-tauri/src/edit/ops/api.rs` - appended by `AddCut` with a fresh id; iterated in `metrics` to compute `kept_ms`
- `src-tauri/src/export/remap.rs` - `TimeMap::build` turns cuts into gaps between kept segments

## ZoomTarget

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ZoomTarget { Cursor, Fixed { x: f32, y: f32 } }
```

Where the camera centers during a zoom event.

- `Cursor` - *track the live cursor position at each frame; appropriate for user-created zooms that should follow the pointer.*
- `Fixed { x, y }` - *anchor to a specific screen-local coordinate (floats); used by `seed.rs` to preserve the auto-zoom anchor from the recording's click position, keeping a re-render byte-identical to the original export.*

### Used by

- `src-tauri/src/edit/ops/api.rs` - `AddZoom` defaults to `Cursor`; `AddZoomFull` also defaults to `Cursor`
- `src-tauri/src/edit/seed.rs` - `zooms_from_regions` always writes `Fixed` to lock the click anchor
- `src-tauri/src/export/render/fromedit.rs` - the compositor branches on this to compute the camera center per frame

## Zoom

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Zoom {
    pub id: String, pub start_ms: u32, pub end_ms: u32,
    pub target: ZoomTarget, pub scale: f32, pub easing: String,
    pub zoom_in_ms: u32, pub zoom_out_ms: u32,
    pub layer: u32,
    pub cam_action: Option<CamZoomAction>,
    pub smart_typing: bool,
    pub easing_out: Option<String>,
}
```

One zoom event in the timeline.

- `id` - *stable string key (e.g. `"z0"`, `"z3"`) used by `UpdateZoom`/`RemoveZoom` to target specific entries without relying on list position.*
- `start_ms` - *frame time at which the zoom-in begins.*
- `end_ms` - *frame time at which the zoom-out completes.*
- `target` - *where the camera should point; see `ZoomTarget`.*
- `scale` - *peak zoom multiplier (e.g. `2.0` = 2x). Passed directly to the compositor.*
- `easing` - *the ZOOM-IN ramp's curve: one of the six named ones (`"smooth"`, `"linear"`, `"spring"`, `"ease_in"`, `"ease_out"`, `"ease_in_out"`) or a parameterised `cubic(...)` / `spring(...)` / `keys(...)` string. Parsed by `easing_from` (`export::render::fromedit`) at render time, canonicalised by `valid_easing` on the way in.*
- `easing_out` - *the ZOOM-OUT ramp's curve (M3). `None` - what every doc written before this field existed loads as, and what `skip_serializing_if` keeps out of a re-saved old doc - means "the same curve as `easing`", which is exactly today's behaviour, so every existing doc reads and renders unchanged. `regions_from_doc` resolves the fallback against the zoom's OWN `easing` (never the config's tuned default), and `CameraSim`'s zoom-out ramp is the one consumer (`export/camera/mod.rs`). Written only when the two ramps genuinely differ: `ops::motion::set_zoom_easing_out` collapses a value equal to `easing` back to `None`, so the doc has exactly one spelling of "both ramps are the same". `LayoutSeg` has carried the same split as `easing`/`easing_out` since exit transitions landed; `CameraMove` keeps its single `easing`, since a keyframe has no exit ramp of its own.*
- `layer` - *priority when this zoom overlaps another in time (higher wins) and the timeline row it renders on. Auto-assigned by `auto_layer` on creation, user-overridable via `UpdateZoom`.*
- `cam_action` - *per-zoom webcam-on-zoom override. `None` inherits `ZoomSettings::resolved_cam_action`. Serialized only when set (`skip_serializing_if`), so re-saving a doc written before this field existed does not start emitting a new key.*
- `smart_typing` - *smart typing duration (owner, 2026-09-14): the end follows the typing after the start. `edit::commands::apply_edit_op` refits `end_ms` through `ops::smart_zoom::refit` whenever this is switched on or the start moves, so the doc always carries a concrete `end_ms` and the timeline, the preview and the export need no new path. `#[serde(default)]`: docs written before it read `false`.*

### Used by

- `src-tauri/src/edit/ops/api.rs` - inserted by `AddZoom`/`AddZoomFull`, mutated by `UpdateZoom`, removed by `RemoveZoom`
- `src-tauri/src/edit/seed.rs` - `zooms_from_regions` constructs the initial list
- `src-tauri/src/export/render/fromedit.rs` - rendered by `CameraSim` per frame
- `src-tauri/src/ai/backend/plan.rs` - AI director reads and writes zoom entries

## Speed

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Speed { pub id: String, pub start_ms: u32, pub end_ms: u32, pub factor: f32 }
```

A playback-speed multiplier applied to a time range.

- `id` - *stable key (e.g. `"s0"`) for plan-based mutation.*
- `start_ms` / `end_ms` - *the span over which the speed change is active.*
- `factor` - *multiplier: `2.0` = double speed, `0.5` = half speed.*

### Used by

- `src-tauri/src/edit/ops/api.rs` - appended by `SetSpeed`
- `src-tauri/src/export/render/fromedit.rs` - applied during frame-time remapping in the compositor

## PanelPose

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct PanelPose { pub cx: f32, pub cy: f32, pub size: f32 }
```

Where one panel sits on the output frame, resolution-independently. Deliberately the SAME three-number vocabulary `CameraMove` already uses, so a webcam keyframe and a layout arrangement describe a pose identically and resolve through the same `rect_from_center` machinery.

- `cx` / `cy` - *the panel's CENTER as fractions of output width/height. Center-based, not top-left, because that is what makes a resize keep the panel where it is - and it matches `CameraMove.x`/`y`.*
- `size` - *the panel's HEIGHT as a fraction of output height.*

**Why no width:** the width is never stored. It is re-derived at resolve time from the panel's own aspect (the screen's is the SOURCE aspect, the cam's is the appearance-configured shape), so no pose - however it was dragged, blended, or hand-edited in the JSON - can ever stretch or squash the content inside a panel. Bounds are enforced on the way in by `edit::ops::arrangement::clamp_pose` (`cx`/`cy` in `0..1`, `size` in `0.05..1.5`).

### Used by

- `src-tauri/src/edit/model.rs` - the two fields of `Arrangement`
- `src-tauri/src/edit/ops/arrangement.rs` - `clamp_pose`; the `SetArrangement` payload
- `src-tauri/src/export/scene/arrangement.rs` - `resolve_arrangement` turns a pose into a `Panel`; `pose_of_panel` derives one back out
- `src/shared/edit.ts` - `PanelPose` TS mirror

## Arrangement

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Arrangement { pub screen: Option<PanelPose>, pub cam: Option<PanelPose> }
```

One `LayoutSeg`'s custom two-panel composition - the T34 model in which a layout segment carries explicit poses instead of only a preset name, and the five presets become one-click starting points that FILL an arrangement.

- `screen` - *pose for the screen (zoomed base layer) panel, or `None` = not shown (resolves to `alpha = 0`).*
- `cam` - *pose for the webcam panel, same `None` rule.*

At least one panel is always `Some`: `edit::ops::arrangement::apply_arrangement` rejects any change that would hide both, so an empty frame is never a savable state (`ClearArrangement` is the way back to the preset). A `None` panel still resolves at the PRESET's rect with `alpha = 0`, so a cross-dissolve into or out of a hidden panel slides it rather than popping it in from a degenerate rect.

### Used by

- `src-tauri/src/edit/model.rs` - the optional `LayoutSeg.arrangement` field
- `src-tauri/src/export/scene/arrangement.rs` - `resolve_arrangement` / `arrangement_of_preset`
- `src-tauri/src/export/scene/layout.rs` - `LayoutTrack::from_segs` resolves it in place of the preset
- `src-tauri/src/export/preview/preview_layouts.rs` - each `LayoutPresetDto` carries the preset's own arrangement, so the editor can convert a preset with one op
- `src/shared/edit.ts` - `Arrangement` TS mirror

## LayoutSeg

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutSeg {
    pub id: String, pub start_ms: u32, pub end_ms: u32, pub layout: String,
    #[serde(default = "default_layout_transition_ms")] pub transition_ms: u32,
    #[serde(default = "default_layout_easing")] pub easing: String,
    #[serde(default)] pub transition_out_ms: u32,
    #[serde(default = "default_layout_easing")] pub easing_out: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arrangement: Option<Arrangement>,
}
```

A time span that uses a named screen layout (e.g. `"screen"`, `"camera"`, `"presenter"`), optionally overridden by explicit panel poses.

- `id` - *stable key (e.g. `"l0"`) used by `UpdateLayoutSeg` to target the segment to update.*
- `start_ms` / `end_ms` - *the span where this layout is active, `[start, end)`. Segments may overlap (latest start wins) and need not tile the clip - a gap resolves to the base `screen` layout.*
- `layout` - *serde wire name of the layout variant (lowercase snake_case); consumed by the compositor to choose the frame composition template.*
- `transition_ms` / `easing` - *the ENTRY cross-fade, which STARTS at `start_ms`. Defaults to 350ms / `"smooth"`.*
- `transition_out_ms` / `easing_out` - *the EXIT cross-fade, which COMPLETES at `end_ms` - symmetric with the entry, so everything a segment does stays inside its own timeline pill. Defaults to `0` = a hard cut, which is exactly what every doc written before exit transitions existed deserializes to, so old docs render bit-identically (pinned by `layout_seg_exit_transition_defaults_to_a_hard_cut_on_missing_fields`). `easing_out` is only meaningful when `transition_out_ms > 0`. See `export/scene/layout.md` for the blend semantics, including why a gapless successor's entry wins the overlap.*
- `arrangement` - *explicit panel poses for this segment (T34). `None` resolves from the `layout` preset exactly as before; `Some` WINS over the preset and demotes `layout` to display-only provenance ("based on Presenter") that still selects the appearance block the panels' radius/ring/shape come from. Resolved by `export::scene::arrangement::resolve_arrangement` inside `LayoutTrack::from_segs`.*

**Back-compat (additive, no doc-version bump).** `#[serde(default)]` makes the field absent-safe, so every pre-T34 `edit.json` loads with `arrangement: None` and renders identically. `skip_serializing_if` closes the other half of the contract: re-saving an untouched old doc does not grow an `"arrangement": null` key, so the bytes on disk are unchanged. Both directions are pinned in `model_arrangement_tests.rs` (`old_layout_seg_json_loads_with_no_arrangement`, `an_untouched_old_seg_re_serializes_byte_identically`). A HIDDEN panel inside a present arrangement is an explicit `null` on the wire, which is why that is distinct from "no arrangement at all".

### Used by

- `src-tauri/src/edit/ops/api.rs` - `SetLayoutSeg` mutates `layout` on a matched entry
- `src-tauri/src/edit/ops/arrangement.rs` - `apply_arrangement` sets/clears `arrangement`
- `src-tauri/src/edit/seed.rs` - `layout_from_actions` builds the initial list from the `SetLayout` action track
- `src-tauri/src/export/render/fromedit.rs` - segment list drives per-frame layout selection in the compositor

## EditDoc

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EditDoc {
    #[serde(default = "oldest_version")] pub version: u32,
    pub trim: Trim,
    #[serde(default)] pub clip_ms: u32,
    pub cuts: Vec<Cut>,
    pub zooms: Vec<Zoom>,
    pub speed: Vec<Speed>,
    pub layout: Vec<LayoutSeg>,
    #[serde(default)] pub effects: Vec<EffectRegion>,
    #[serde(default)] pub camera_moves: Vec<CameraMove>,
    pub aspect: crate::export::types::Aspect,
    pub settings: crate::settings::model::Settings,
    #[serde(default)] pub captions: Vec<Caption>,
}
```

Root of `edit.json`. Carries the complete editor state for one recording project.

**One clock.** Every region list in the doc - `zooms`, `effects`, `layout`, `camera_moves` - stores OUTPUT time (ms, 0 = the first video frame), the same base the editor timeline, the TS preview and `FramePose::out_t` use. Raw recorded streams (`events.json`, `actions.json`, `cursor.json`) stay on the EVENT clock and are converted at the boundary (`seed::actions_on_output_clock`, `FrameRenderer::click_track`, `cursor_kinds`). This is what `DOC_VERSION` 2 guarantees; v1 stored `effects`/`layout` on the event clock.

- `version` - *schema version; `DOC_VERSION` for anything this build writes. `#[serde(default = "oldest_version")]` overrides the container default so a file WITHOUT the key reads as `1` (the oldest schema) and gets migrated - with the container default it would read as `DOC_VERSION` and silently skip every migration. `edit::migrate::migrate` upgrades older docs on load.*
- `trim` - *clip in/out bounds; the only time bounds that affect the exported file's duration.*
- `clip_ms` - *the recording's TRUE full duration (ms), independent of `trim`. The upper bound `edit::ops::region::dur_bound` clamps new/moved regions against, so trimming the clip no longer collapses a region added past the trim point. `#[serde(default)]` so a pre-existing doc loads it as `0` ("not yet known"); `seed::build_default` seeds it and `edit::migrate::migrate` backfills it on every load where it is still `0`, regardless of `version`.*
- `cuts` - *ordered list of removed spans within the trim window.*
- `zooms` - *ordered list of zoom events; `api.rs` manages ids; the renderer tolerates any order.*
- `speed` - *ordered list of speed-change segments.*
- `layout` - *ordered, non-overlapping layout segments covering `[0, trim.out_ms]`.*
- `effects` - *ordered list of editable effect regions (v1: Spotlight); `#[serde(default)]` for back-compat. See `EffectRegion`.*
- `camera_moves` - *ordered list of webcam PiP keyframes; `#[serde(default)]` so a pre-existing `edit.json` with no `camera_moves` loads as an empty `Vec`, which the exporter/preview treat as "no override" (byte-identical to today).*
- `aspect` - *output frame aspect ratio; `#[serde(default)]` so a pre-existing `edit.json` with no `aspect` loads as `Aspect::Source` - today's behavior exactly. See `export::types::Aspect`.*
- `settings` - *snapshot of the user's `Settings` at the time the doc was seeded; preserves the zoom config and theme for a re-render even if the user later changes settings.*
- `captions` - *the spoken-caption track (see `edit::captions::Caption`), OUTPUT-clock like every other region list here. `#[serde(default)]` so a doc written before captions existed loads with an empty track.*

### Used by

- `src-tauri/src/edit/ops/api.rs` - `apply` mutates it; `metrics` reads it
- `src-tauri/src/edit/commands.rs` - all three Tauri commands return or accept `EditDoc`
- `src-tauri/src/edit/seed.rs` - `load_or_seed` and `build_default` construct it
- `src-tauri/src/export/render/fromedit.rs` - consumed to drive the compositor
- `src-tauri/src/ai/backend/plan.rs` / `src-tauri/src/ai/commands.rs` - AI director reads and patches it

## EditDoc::save

```rust
pub fn save(&self, path: &std::path::Path) -> std::io::Result<()>
```

Serializes the doc to pretty-printed JSON and writes it atomically to `path`: the bytes go to a temp sibling (`crate::process::proc::tmp_sibling`) first, then `std::fs::rename` moves it into place. A crash or power loss mid-write leaves the old `edit.json` untouched (the rename either fully happens or not at all) instead of a half-written, truncated file - the failure mode `EditDoc::load` used to see as silent corruption.

### Inputs

- `&self` - the doc to persist. *Why pretty-print:* `edit.json` is human-readable and diff-friendly in git.*
- `path: &std::path::Path` - destination file. *Why `Path` not `ProjectPaths`:* the method is on the model, which has no knowledge of project layout; callers supply the resolved path.*

### Returns

`Ok(())` on success; `Err(io::Error)` on serialization, temp-file write, or rename failure. On success no temp sibling is left behind; `rename` replaces an existing `path` in place (verified on Windows - no need to remove the destination first).

### Behaviors

- `round_trip_save_load` - a full `EditDoc` with all fields populated serializes and deserializes without data loss.
- `save_leaves_no_tmp_sibling_on_success` - after a successful save, the directory contains only the target file, no leftover temp sibling.
- `save_overwrites_an_existing_file` - saving over a path that already holds a doc replaces it (the common per-edit-op case).

## EditDoc::assign_missing_ids

```rust
pub fn assign_missing_ids(&mut self)
```

Cuts saved before they had ids get `c{n}` ones, `n` counting up from the highest live id. Called by `load` so a pre-remap `edit.json` comes back addressable without a migration; `timeops_tests::a_doc_saved_before_cut_ids_loads_with_ids_assigned` pins the numbering.

## EditDoc::load

```rust
pub fn load(path: &std::path::Path) -> Option<EditDoc>
```

Reads and deserializes `edit.json` at `path`. A missing file and a corrupt (unparseable) file both return `None`, so the caller's reseed path (`load_or_seed`) runs either way - but they are NOT treated the same on disk: a read failure (no file) is left alone, while a parse failure delegates to `process::proc::preserve_corrupt(path, &e)`, which renames the bad file aside to `<path>.corrupt` (overwriting any older `.corrupt` from a previous crash) and `eprintln!`s the parse error, so a reseed never silently destroys the user's actual edit - the original bytes survive on disk for recovery. (`settings::store::load_from` shares this same helper for `config.json`, bug-sweep-2 M3.)

### Inputs

- `path: &std::path::Path` - file to read. *Why `Option` rather than `Result`:* callers treat missing-or-corrupt as "seed needed", not an error, so a silent `None` matches the intended flow.*

### Returns

`Some(EditDoc)` if the file exists and is valid JSON; `None` on any I/O or parse failure.

### Behaviors

- `load_missing_path_is_none` - a path that does not exist returns `None`.
- `partial_json_fills_defaults` - JSON with only a `zooms` key fills all other fields from `Default`.
- `zoom_target_fixed_serializes_with_xy` - `ZoomTarget::Fixed` round-trips with `"fixed"`, `"x"`, and `"y"` keys present.
- `aspect_missing_field_defaults_to_source` - JSON without an `aspect` key loads `Aspect::Source` (back-compat).
- `clip_ms_missing_field_defaults_to_zero` - JSON without a `clip_ms` key loads it as `0` (back-compat; `edit::migrate::migrate` backfills it).
- `aspect_round_trips_through_json` - a non-default `Aspect` round-trips through `EditDoc` serialization.
- `load_on_truncated_json_returns_none_and_preserves_original_bytes` - a parse failure returns `None` and the original bytes end up unmodified at `<path>.corrupt`.
- `load_on_truncated_json_overwrites_an_older_corrupt_file` - a stale `.corrupt` sibling from an earlier crash does not block preserving the new one.

