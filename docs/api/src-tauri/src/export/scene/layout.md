# src-tauri/src/export/scene/layout.rs

Resolves the active `Scene` at any video timestamp from the recording's `SetLayout` action track, smoothly cross-fading between layout presets over `transition_ms`. Also re-anchors zoom regions from screen-local coordinates into the screen panel coordinate system active at each zoom's start time.

## LayoutTrack

```rust
pub struct LayoutTrack { segs: Vec<Seg>, base: Scene }

struct Seg { start_ms: u32, end_ms: u32, scene: Scene, transition_ms: u32, easing: Easing,
    transition_out_ms: u32, easing_out: Easing }
```

Holds the resolved layout segments for one export. Built once (`new` from recorded actions, or `from_segs` from `doc.layout`) and queried per output frame in `scene_at`.

### Fields

- `segs: Vec<Seg>` - *segments sorted by `start_ms`. Each is active only INSIDE `[start_ms, end_ms)` and carries its own cross-fade feel in AND out.*
- `base: Scene` - *the `screen` layout. OUTSIDE every segment - a gap, or before the first / after the last - this is what resolves, the free-pill "empty means default" model.*
- `Seg::transition_ms` / `easing` - *the ENTRY blend, which STARTS at `start_ms`.*
- `Seg::transition_out_ms` / `easing_out` - *the EXIT blend, which COMPLETES at `end_ms` (symmetric with the entry). `0` is a hard cut - the value every doc written before exit transitions existed deserializes to, so old docs render bit-identically.*

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - creates a `LayoutTrack` and calls `scene_at` for every output frame.
- `src-tauri/src/export/scene/layout.rs` - `anchor_regions` calls `scene_at` to find the screen panel rect at each zoom's start time.
- `src-tauri/src/edit/seed.rs` - reads layout changes from a `LayoutTrack` when seeding the editor doc.

## LayoutTrack::new

```rust
pub fn new(actions: &[ActionEvent], app: &AppearanceSettings, ow: u32, oh: u32,
           sw: u32, sh: u32, transition_ms: u32) -> Self
```

Builds the switch list from the `SetLayout` actions in the recording.

### Inputs

- `actions: &[ActionEvent]` - full action track. *Why:* only `ActionKind::SetLayout(id)` events are extracted; all other kinds are ignored.*
- `app: &AppearanceSettings` - user appearance settings for each layout preset. *Why:* each `LayoutId` resolves to a `Scene` via `app.for_id(id)` -> `layout_for` -> `resolve`.*
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* `layout_for` and `overlay_for` use these to compute panel pixel rects at the output resolution.*
- `sw: u32`, `sh: u32` - source screen dimensions. *Why:* `resolve` uses these to place the screen panel relative to the output canvas.*
- `transition_ms: u32` - cross-fade duration. *Why:* stored for `scene_at`.*

### Returns

`LayoutTrack` with `switches` starting at `(0, Screen)` followed by one entry per `SetLayout` action.

### Implementation

1. Build a closure `scene_for(id)` that calls `app.for_id(id)` -> `layout_for(ma, ow, oh)` -> `overlay_for(ma, ow, oh, true)` -> `resolve(id, layout, overlay, sw, sh)`.
2. Initialize `switches = vec![(0, scene_for(LayoutId::Screen))]`. *Why Screen as base:* recordings always begin with the default Screen layout.*
3. Walk `actions`; for each `ActionKind::SetLayout(id)`, push `(a.t, scene_for(id))`.

## LayoutTrack::scene_at

```rust
pub fn scene_at(&self, t_ms: u32) -> Scene
```

Returns the resolved `Scene` at `t_ms`: the active segment (else `base`), cross-faded in from what preceded it and out toward what follows it.

### Inputs

- `t_ms: u32` - OUTPUT time in milliseconds (0 = the first video frame). *Why:* the track's segments come from `doc.layout`, and every `EditDoc` region list is stored on the output clock, so the query must use the same base. `FrameRenderer::step_camera` passes `pose.out_t`; `anchor_regions` passes a zoom's `start_ms`, which is output time too. The recorded-action fallback track (`LayoutTrack::new`) is shifted onto the output clock by its builder (`EditState::load`) so this holds for both construction paths.*

### Returns

`Scene` - fully resolved layout with screen panel rect, camera panel rect, and alphas.

### Implementation

1. `active_idx(t_ms)` - the LAST-starting segment containing `t_ms`, so an overlap resolves to the newer one. `None` (a gap) returns `base` immediately.
2. **Entry**, checked FIRST: if `transition_ms > 0` and `elapsed = t_ms - start_ms` is inside it, blend `raw_scene(start_ms - 1)` -> this segment over `ease(easing, elapsed / transition_ms)`. Checking entry first is what makes a segment shorter than its own two transitions still resolve deterministically - it eases in, never out.
3. **Exit**: if `transition_out_ms > 0` and `t_ms >= end_ms - transition_out_ms`, ask `successor(end_ms)` what this segment hands off to and whether that successor's own entry is still running there.
   - If the successor's entry IS running, it **wins** the overlap: the exit stands down and the segment's own scene is returned. One blend at a time - no double-blend, and gapless back-to-back segments stay bit-identical to their pre-exit-transition behavior (the successor's entry already blends FROM this segment, so it was never a pop).
   - Otherwise blend this segment -> the successor over `ease(easing_out, (t_ms - exit_from) / transition_out_ms)`. The fraction reaches exactly `1` at `end_ms`, so the exit lands ON the successor's pose rather than jumping to it. This is what smooths a segment falling back into a gap, or into a successor that hard-cuts in.
4. Otherwise return the segment's own scene.

*Why the exit is expressed as "completes at `end_ms`" rather than "starts at `end_ms`":* the pill on the timeline is an honest bound - everything the segment does happens inside its own span, exactly like a zoom's `zoom_out_ms`.

### Behaviors worth knowing

- `no_actions_is_screenfocus_everywhere` - no `SetLayout` actions -> every timestamp returns the default `Screen` scene.
- `switch_transitions_then_settles` - at `t=999` the scene is Screen; transition starts at `t=1000`; by `t=1400` it has settled to Camera (400ms transition). The midpoint `t=1200` has `screen.rect.w` strictly between the two presets.
- `latest_switch_wins` - with switches at `t=100` and `t=200`, querying `t=10000` returns the `t=200` preset.
- `default_zero_exit_is_the_historical_hard_cut` - the pre-exit-transition gap fixture, sample for sample.
- `exit_blend_completes_exactly_at_end_ms` - the window opens at `f=0`, is strictly between the two poses mid-way, and the last active sample is within 0.05px of what `end_ms` resolves to; the blend fraction itself is exactly `1.0` at `end_ms`.
- `a_gapless_successors_entry_wins_the_overlap` - the exit is suppressed against a successor with its own entry, and DOES run against one that hard-cuts in.
- `a_segments_own_entry_and_exit_are_fitted_into_a_span_too_short_for_both` - a 300ms segment asking for 300ms of each becomes a symmetric 150/150 bump: the entry arrives at the midpoint, the exit carries it back before `end_ms`.
- `a_segment_shorter_than_its_transitions_hands_over_without_a_jump` - the boundary case the fitting exists for. A 200ms segment with a 350ms entry used to jump ~950px in one frame at its `end_ms` (an ordinary step at that point is ~193px), because it never reached its own scene while `raw_scene` handed that unreached scene to the next segment's entry regardless.
- `an_entry_and_exit_that_would_overlap_are_fitted_instead` - 700ms of transition inside a 400ms segment used to run the exit UNDER the entry, so the instant the entry expired the panel lurched ~1794px in a single millisecond.

## resolve_seg_scene

```rust
pub fn resolve_seg_scene(s: &LayoutSeg, app: &AppearanceSettings, ow: u32, oh: u32, sw: u32, sh: u32) -> Scene
```

One `LayoutSeg`'s resolved `Scene`: its POSES when it carries an `arrangement` (via `scene::arrangement::resolve_arrangement`; its `layout` name then only picks the `ModeAppearance` block, so the same poses under a different provenance can legitimately differ in cam shape/ring - see `the_layout_name_still_selects_the_appearance_block`), else its preset's `Scene` directly.

**The single definition, used from two places (T34 L2).** `from_segs` (below) calls this once per segment to build the export/preview-track path; `preview_layouts` (`preview/preview_layouts.md`) calls it again through `FrameRenderer::resolve_seg` (`render/accessors.md`) to report a posed segment's true panels to the editor's live canvas preview. Factored out so a segment's live preview and what the export actually draws can never diverge onto two pose-math paths - there is exactly one function that turns a `LayoutSeg` into a `Scene`.

## from_segs

```rust
pub fn from_segs(segs: &[crate::edit::model::LayoutSeg], app: &AppearanceSettings,
                 ow: u32, oh: u32, sw: u32, sh: u32) -> Self
```

The EDITED path: one `Seg` per `LayoutSeg`, each carrying its own span, entry feel and exit feel (both easing strings go through `easing_from`, so a custom `cubic(...)` curve works for either). Gaps between segments fall back to the base `screen`. Segments are sorted by `start_ms` on construction, so `active_idx`'s "last one wins" is a genuine latest-start rule.

**Transitions are fitted into the span first.** Each segment's `(transition_ms, transition_out_ms)` goes through `fit_durations` (`camera/mod.md`) against `end_ms - start_ms` before being stored, exactly as a zoom's ramps are fitted into its pill - so `Seg` only ever holds durations guaranteed to finish inside the segment, and every reader (`scene_at`'s two branches, `successor`'s "is the next entry still running" test) sees the same fitted value. Unfitted, a segment shorter than its own entry never reached its own scene, yet `raw_scene` handed that unreached scene to whatever blended off it next, so the frame JUMPED at the boundary; and an entry+exit that together outlasted the span ran the exit underneath the entry, lurching most of the way to the successor the moment the entry expired. `LayoutTrack::new` fits the same way, for two recorded switches closer together than the global transition.

Each segment's `Scene` comes from `resolve_seg_scene` (above), called once, up front: `scene_at` then blends already-resolved scenes, so an arrangement<->preset cross-fade is bit-for-bit the same `Scene::lerp` a preset<->preset one is, with no second blend path to keep in sync. Pinned by `a_preset_to_arrangement_crossfade_is_the_same_lerp_as_preset_to_preset` and `an_arrangement_to_preset_exit_blend_lands_on_the_successor_at_end_ms` (`layout_arrangement_tests.rs`).

## successor

```rust
fn successor(&self, end_ms: u32) -> (Scene, bool)
```

What a segment hands off to at its `end_ms` - the next segment if the two are gapless, else the base `screen` - plus whether that successor's OWN entry blend is still running at that instant. The `bool` is the overlap rule: `true` means the successor's entry wins and the exit must not run.

## anchor_frame

```rust
pub fn anchor_frame(raw: &[ZoomRegion], scene: &Scene, out: &mut Vec<ZoomRegion>)
```

Re-anchors every zoom region into THIS frame's screen panel, writing the result into `out` - a scratch buffer the caller keeps (`FrameRenderer::frame_regions`), so a 60 fps walk allocates nothing. Each region is re-anchored with `to_panel(anchor, scene.src, scene.screen.rect)`: the panel a region anchors into AND the crop rect it anchors through (a mid-take display switch's fitted sub-rect) both come from the frame's own resolved scene.

**Why per frame, not per region (2026-09-14).** This used to be `anchor_regions`, run once per region at the region's `start_ms` on the assumption that layouts do not change mid-zoom. They do - a layout segment boundary, an arrangement, a display switch - and when the panel moved out from under a pinned aim the camera kept zooming into where the content HAD been, which the owner reported as the layout going haywire mid-zoom. Anchoring from the frame's scene carries the aim with the panel; `CameraSim`'s own follow damping turns the moving target into a smooth pan.

### Inputs

- `raw: &[ZoomRegion]` - zoom regions with RAW canvas-space anchors (`fromedit::regions_from_doc`, ultimately `autozoom::generate` / `manual::from_actions` / the doc's `Fixed` targets). *Why:* those produce anchors in full-canvas coordinates; the camera needs them in output px relative to the screen panel, which varies by layout and by frame.*
- `scene: &Scene` - the frame's resolved scene (`SpanTrack::frame_at` in `step_camera`), whose `screen.rect` and `src` place the anchor.
- `out: &mut Vec<ZoomRegion>` - cleared and refilled, same length and order as `raw` (`CameraSim` keeps a driver INDEX across frames, so the order must not change).

### Implementation

1. `out.clear()`.
2. For each region `r`, push `ZoomRegion { anchor: to_panel(r.anchor, scene.src, scene.screen.rect), ..*r }`.

### Behaviors (`layout_tests.rs`)

- `anchor_frame_follows_the_panel_frame_by_frame` - the same raw anchor lands at the panel's centre for a full-frame panel and at the centre of a half-size, offset panel when that is the frame's scene; the buffer is reused between calls.
