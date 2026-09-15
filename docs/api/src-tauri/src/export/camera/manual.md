# src-tauri/src/export/camera/manual.rs

Converts `ZoomHoldStart` / `ZoomHoldEnd` action pairs into `ZoomRegion` values for the manual-zoom effect. Each matched pair produces one region anchored at the cursor position at the moment the key was pressed; unmatched `ZoomHoldStart` events (hold still active at recording end) produce no region.

## from_actions

```rust
pub fn from_actions(actions: &[ActionEvent], events: &[MouseEvent], screen: &ScreenInfo, cfg: &ZoomConfig) -> Vec<ZoomRegion>
```

Scans `actions` for `ZoomHoldStart` / `ZoomHoldEnd` pairs and emits one `ZoomRegion` per matched pair.

### Inputs

- `actions: &[ActionEvent]` - full action track. *Why:* only `ZoomHoldStart` and `ZoomHoldEnd` kinds are consumed; all other kinds are skipped.*
- `events: &[MouseEvent]` - full mouse event log. *Why:* `cursor_at` searches this to find the cursor position at or just before each `ZoomHoldStart`, so the zoom anchors where the user was pointing when the key was pressed.*
- `screen: &ScreenInfo` - capture screen geometry. *Why:* `to_frame` needs the screen rect to convert raw mouse coordinates to frame-local pixel coordinates.*
- `cfg: &ZoomConfig` - zoom settings (scale, `zoom_in_ms`, `zoom_out_ms`, easing). *Why:* manual zoom uses the same tunables as click-zoom; no separate config is needed.*

### Returns

`Vec<ZoomRegion>` - one entry per matched pair, in time order. Anchors are screen-local; `layout::anchor_frame` re-anchors them into each frame's screen panel.

### Implementation

1. Walk `actions` with a `start: Option<u32>` accumulator.
2. On `ZoomHoldStart`: record `start = Some(a.t)`.
3. On `ZoomHoldEnd`: if `start` is `Some(s)`, call `cursor_at(events, screen, s)` for the anchor - the last mouse sample at or before `s`, or screen center if none. `cursor_at` assumes `events` is in ascending capture-time order; it does not re-sort. Push `ZoomRegion { start_ms: s, end_ms: max(a.t, s+1) + cfg.zoom_out_ms, zoom_in_ms: cfg.zoom_in_ms, zoom_out_ms: cfg.zoom_out_ms, target_scale: cfg.target_scale, anchor, easing: cfg.easing, easing_out: cfg.easing }`. (`easing_out` equals `easing` here for the same reason it does in `autozoom`: a generated region has one curve, and only the editor ever splits the two ramps.) *Why `max(a.t, s+1)`:* guarantees `end_ms > start_ms` even for an instantaneous tap, preventing a zero-duration region.*
4. Clear `start`; continue. Other action kinds are skipped.

### Behaviors worth knowing

- `hold_pair_makes_one_region_anchored_at_press` - hold from 1000ms to 2000ms with cursor at `(500, 300)` at `t<=1000` produces one region: `start=1000`, `end=2000+zoom_out_ms`, `anchor=(500, 300)`.
- `unpaired_start_makes_no_region` - a `ZoomHoldStart` with no matching `ZoomHoldEnd` yields an empty vec.
- `two_holds_make_two_regions` - two hold pairs yield two regions; with no mouse events the anchor defaults to screen center `(960, 540)`.
