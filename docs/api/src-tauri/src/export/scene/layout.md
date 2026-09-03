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
- `a_segments_own_entry_beats_its_own_exit_when_they_overlap` - a 300ms segment with 300ms of each still only eases in.

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

Each segment's `Scene` comes from `resolve_seg_scene` (above), called once, up front: `scene_at` then blends already-resolved scenes, so an arrangement<->preset cross-fade is bit-for-bit the same `Scene::lerp` a preset<->preset one is, with no second blend path to keep in sync. Pinned by `a_preset_to_arrangement_crossfade_is_the_same_lerp_as_preset_to_preset` and `an_arrangement_to_preset_exit_blend_lands_on_the_successor_at_end_ms` (`layout_arrangement_tests.rs`).

## successor

```rust
fn successor(&self, end_ms: u32) -> (Scene, bool)
```

What a segment hands off to at its `end_ms` - the next segment if the two are gapless, else the base `screen` - plus whether that successor's OWN entry blend is still running at that instant. The `bool` is the overlap rule: `true` means the successor's entry wins and the exit must not run.

## anchor_regions

```rust
pub fn anchor_regions(raw: Vec<ZoomRegion>, track: &LayoutTrack, sw: u32, sh: u32) -> Vec<ZoomRegion>
```

Re-anchors each zoom region into the screen panel coordinate system that is active at the region's start time.

### Inputs

- `raw: Vec<ZoomRegion>` - zoom regions with screen-local anchors (from `autozoom::generate` or `manual::from_actions`). *Why:* those modules produce anchors in full-screen coordinates; the renderer needs them relative to the screen panel rect, which varies by layout.*
- `track: &LayoutTrack` - the layout track for the current export. *Why:* `track.scene_at(r.start_ms)` yields the screen panel rect at the moment each zoom begins. Both sides of that call are output time (a zoom's `start_ms` and the track's segments), so the panel looked up really is the one active when the zoom fires.*
- `sw: u32`, `sh: u32` - source screen dimensions. *Why:* `to_panel` uses these to normalize the anchor into the panel rect.*

### Returns

`Vec<ZoomRegion>` - same regions with `anchor` remapped to panel-local coordinates; all other fields unchanged.

### Implementation

1. For each region `r`, get `panel = track.scene_at(r.start_ms).screen.rect`.
2. Call `to_panel(r.anchor, sw, sh, panel)` -> new anchor.
3. Return `ZoomRegion { anchor: panel_anchor, ..r }`. *Why at `start_ms`:* the zoom anchors at the layout active when it begins; layouts do not change mid-zoom.*
