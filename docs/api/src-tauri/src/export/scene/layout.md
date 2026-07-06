# src-tauri/src/export/scene/layout.rs

Resolves the active `Scene` at any video timestamp from the recording's `SetLayout` action track, smoothly cross-fading between layout presets over `transition_ms`. Also re-anchors zoom regions from screen-local coordinates into the screen panel coordinate system active at each zoom's start time.

## LayoutTrack

```rust
pub struct LayoutTrack {
    switches: Vec<(u32, Scene)>,
    transition_ms: u32,
}
```

Holds the precomputed sequence of layout changes for one export. Initialized once in `new` and queried per output frame in `scene_at`.

### Fields

- `switches: Vec<(u32, Scene)>` - *time-ordered list of `(start_ms, scene)` pairs. Always begins with `(0, Screen)` so every timestamp has a valid base scene. Each `SetLayout` action appends one entry.*
- `transition_ms: u32` - *cross-fade duration in milliseconds. `0` disables interpolation (hard cut).*

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

Returns the interpolated `Scene` at `t_ms`, cross-fading from the previous switch over the first `transition_ms` after a layout change.

### Inputs

- `t_ms: u32` - video timestamp in milliseconds. *Why:* called once per output frame by the exporter.*

### Returns

`Scene` - fully resolved layout with screen panel rect, camera panel rect, and alphas.

### Implementation

1. Find `i = rposition` of the last switch with `start <= t_ms`. *Why `rposition`:* the last qualifying entry is the active one; scanning backwards stops at the first match.*
2. If `i == 0` or `transition_ms == 0`, return the current scene immediately.
3. Compute `elapsed = t_ms - start`. If `elapsed >= transition_ms`, return the current scene (transition settled).
4. Compute `t = ease(Easing::Smooth, elapsed / transition_ms)`. *Why Smooth:* a cubic ease avoids the visual pop of a linear blend when panels change size.*
5. Return `Scene::lerp(&prev, &cur, t)`.

### Behaviors worth knowing

- `no_actions_is_screenfocus_everywhere` - no `SetLayout` actions -> every timestamp returns the default `Screen` scene.
- `switch_transitions_then_settles` - at `t=999` the scene is Screen; transition starts at `t=1000`; by `t=1400` it has settled to Camera (400ms transition). The midpoint `t=1200` has `screen.rect.w` strictly between the two presets.
- `latest_switch_wins` - with switches at `t=100` and `t=200`, querying `t=10000` returns the `t=200` preset.

## anchor_regions

```rust
pub fn anchor_regions(raw: Vec<ZoomRegion>, track: &LayoutTrack, sw: u32, sh: u32) -> Vec<ZoomRegion>
```

Re-anchors each zoom region into the screen panel coordinate system that is active at the region's start time.

### Inputs

- `raw: Vec<ZoomRegion>` - zoom regions with screen-local anchors (from `autozoom::generate` or `manual::from_actions`). *Why:* those modules produce anchors in full-screen coordinates; the renderer needs them relative to the screen panel rect, which varies by layout.*
- `track: &LayoutTrack` - the layout track for the current export. *Why:* `track.scene_at(r.start_ms)` yields the screen panel rect at the moment each zoom begins.*
- `sw: u32`, `sh: u32` - source screen dimensions. *Why:* `to_panel` uses these to normalize the anchor into the panel rect.*

### Returns

`Vec<ZoomRegion>` - same regions with `anchor` remapped to panel-local coordinates; all other fields unchanged.

### Implementation

1. For each region `r`, get `panel = track.scene_at(r.start_ms).screen.rect`.
2. Call `to_panel(r.anchor, sw, sh, panel)` -> new anchor.
3. Return `ZoomRegion { anchor: panel_anchor, ..r }`. *Why at `start_ms`:* the zoom anchors at the layout active when it begins; layouts do not change mid-zoom.*
