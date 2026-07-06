# src-tauri/src/export/camera/autozoom.rs

Generates automatic zoom regions from recorded mouse clicks (and optionally typing). This is the core auto-zoom heuristic: it decides WHEN to zoom in, WHERE to anchor, and HOW LONG to hold before releasing, purely from the click/keystroke timeline. Pure and deterministic - identical inputs always yield identical regions, so exports are reproducible.

## generate

```rust
pub fn generate(events: &[MouseEvent], screen: &ScreenInfo, cfg: &ZoomConfig, typing: &[u32], smart: bool) -> Vec<ZoomRegion>
```

Scans the click timeline and emits one `ZoomRegion` per detected activity burst.

### Inputs (what, and why it is needed)

- `events: &[MouseEvent]` - the full mouse log; only `EventKind::Down` events are used (moves/ups do not signal intent). Assumed ascending in `t`. *Why:* a click is the strongest signal that the user is acting on something worth focusing on.
- `screen: &ScreenInfo` - capture geometry (w/h/origin). *Why:* the click is in screen pixels, but a `ZoomRegion.anchor` is in frame-local pixels; the conversion via `coordmap::to_frame` needs the screen rect.
- `cfg: &ZoomConfig` - all tunables (see below). *Why:* keeping policy (durations, thresholds) in one struct lets the user's settings drive behavior with no code change, and keeps the algorithm free of magic numbers.
- `typing: &[u32]` - keystroke timestamps (ms). *Why:* a user may stop clicking but keep typing (e.g. filling a field while zoomed); those keystrokes should keep the zoom alive. Ignored unless `smart`.
- `smart: bool` - whether typing extends a hold. *Why:* an explicit toggle so "smart zoom" turns on/off without a second code path.

### Which `cfg` fields this function reads, and why

- `clicks_to_trigger` - clicks (within `merge_window_ms`) required to START a zoom. *Why:* lets the user demand a quick double-click to trigger, avoiding a zoom on every stray click. `max(1)` guards a 0 setting.
- `merge_window_ms` - the window those trigger-clicks must fall inside. *Why:* separates a deliberate rapid multi-click from two unrelated clicks.
- `idle_release_ms` - the largest gap between consecutive activities that still counts as "still active". *Why:* defines when the user has gone idle so the zoom releases after a quiet stretch instead of staying glued.
- `zoom_in_ms` / `zoom_out_ms` / `target_scale` / `easing` - copied verbatim onto each region. *Why:* a region is self-describing for the renderer, so no second lookup is needed downstream.

### Returns

`Vec<ZoomRegion>` in ascending time, non-overlapping. Each region has `start_ms` at the burst's first click and `end_ms = last_activity + idle_release_ms + zoom_out_ms` (hold through the idle grace, then ease out). Empty when no burst meets the trigger. Anchors are screen-local; the exporter later re-anchors them into the active screen panel via `layout::anchor_regions`.

### Implementation

1. Filter `events` to `Down` clicks (`downs`).
2. Walk `downs` with index `i`. Test whether `need = clicks_to_trigger` clicks from `i` fall within `merge_window_ms` (`downs[i+need-1].t - downs[i].t <= merge_window_ms`). If not, advance `i` by one - a lone or slow click never triggers.
3. On a trigger, anchor at the FIRST click of the cluster. *Why first, not last:* intent is set by where the burst began.
4. Extend the hold past the cluster: gather later click times, plus typing times when `smart`, keep those `>= cluster_last`, sort, then chain via `extend_hold` - each activity within `idle_release_ms` of the previous edge pushes the hold forward. *Why:* a zoom should track continuous activity (more clicks, typing) and release only after a real idle gap, regardless of on-screen distance.
5. Advance `i` past every click `<= last_t` (the final activity, not `end_ms`) so the consumed activity cannot re-trigger a second overlapping zoom.
6. Push the region; continue.

### Behaviors worth knowing (each pinned by a unit test)

- A far-away click SOON after extends the active zoom instead of starting a new one - holds are time-driven, not distance-driven (`click_while_zoomed_extends_regardless_of_distance`).
- Clicks far apart in time make separate, each-releasing zooms with a zoomed-out gap (`idle_clicks_release_between_clusters`).
- With `clicks_to_trigger = 2`, a lone click yields nothing; two must fall within `merge_window_ms` (`clicks_to_trigger_two_*`).
- Typing extends the hold only when `smart` (`typing_extends_the_hold_when_smart` / `typing_ignored_when_not_smart`).

## extend_hold

```rust
fn extend_hold(start: u32, acts: &[u32], idle_ms: u32) -> u32
```

Private helper: from `start`, repeatedly jump to the next activity within `idle_ms` of the current hold edge, returning the last time reached.

- `start` - the hold's initial edge (the cluster's last click time).
- `acts` - candidate activity times (later clicks + typing). **Must be sorted ascending**: times before the current edge are skipped, and the first gap larger than `idle_ms` stops the chain (`break`).
- `idle_ms` - the maximum tolerated gap (`cfg.idle_release_ms`).

*Why a separate function:* isolating the "chain activities by idle gaps" rule keeps `generate` readable and makes the hold logic unit-testable on its own. Uses `saturating_sub`, so equal or out-of-order timestamps can never underflow.
