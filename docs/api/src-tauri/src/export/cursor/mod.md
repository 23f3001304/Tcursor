# src-tauri/src/export/cursor/mod.rs

Stateful cursor position tracker that interpolates between throttled mouse samples and then applies an exponential low-pass filter so small jitter does not cause the on-screen cursor to jump in the rendered video. Owns its event log so it can live inside the `FrameRenderer` as the single owner of the events (also the source the FX layer reads). The tracker advances forward-only through the log for amortized O(1) lookups.

## Cursor

```rust
pub struct Cursor {
    events: Vec<MouseEvent>,
    screen: ScreenInfo,
    idx: usize,
    sx: f32,
    sy: f32,
    primed: bool,
}
```

Stateful cursor tracker that owns the mouse log and screen geometry.

- `events` - the full mouse log in ascending time order. *Why owned (not borrowed):* the tracker lives inside `FrameRenderer` for the whole render; a borrow would make the renderer self-referential. Owning the single copy also lets FX rendering read it via `events()` instead of a second copy of the log.
- `screen` - capture geometry (width, height, origin). *Why:* `raw_at` calls `to_frame`, which needs the screen rect to convert screen-space coordinates to frame-local pixels.
- `idx` - the index of the last event at or before the most recently queried time. *Why mutable:* enables a forward-only scan rather than a binary search on every frame, amortising cost to O(1) per call when frames are queried in order.
- `sx`, `sy` - smoothed position accumulators in frame-local float pixels. *Why floats:* the low-pass filter accumulates sub-pixel movement; rounding only happens on output.
- `primed` - whether `sx`/`sy` have been initialised. *Why:* the first call must seed the accumulator from the raw position rather than lerp toward it from (0, 0), which would produce an unwanted glide-in at the start of the recording.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer` owns one `Cursor`, calls `at` once per rendered frame to place the synthetic cursor / anchor effects, and exposes `events()` to the FX layer.

## Cursor::new

```rust
pub fn new(events: Vec<MouseEvent>, screen: ScreenInfo) -> Self
```

Constructs a `Cursor` that takes ownership of the log, with all smoothing state at zero.

### Inputs

- `events: Vec<MouseEvent>` - the full mouse log, assumed sorted ascending by `t`. *Why ascending:* the forward-only `idx` scan skips unseen events; out-of-order events would cause missed samples. *Why by value:* the renderer moves its single copy of the log in here.
- `screen: ScreenInfo` - capture geometry, owned for the tracker's lifetime so `raw_at` can convert on every frame.

### Returns

`Cursor` with `idx = 0`, `sx = 0.0`, `sy = 0.0`, `primed = false`.

## Cursor::events

```rust
pub fn events(&self) -> &[MouseEvent]
```

Borrows the owned mouse log. *Why it exists:* `FrameRenderer` keeps a single copy of the events inside its `Cursor`; FX rendering needs the same slice, so it reads it through this accessor rather than the renderer holding a second `Vec`.

### Returns

`&[MouseEvent]` - the events passed to `new`, in ascending time order.

## Cursor::clicks

```rust
pub fn clicks(&self) -> Vec<(u32, f32, f32)>
```

Click (mouse-down) positions as `(event_time_ms, x, y)` where `x`/`y` are 0..1 fractions of the screen content - the same coordinate basis as the smoothed cursor (`to_frame`, then divide by the screen size). *Why it exists:* the editor preview draws click ripples; sharing the cursor's basis means a ripple lands exactly where the cursor clicked. `FrameRenderer::click_track` shifts these event times to output time.

### Returns

`Vec<(u32, f32, f32)>` - one tuple per `EventKind::Down` event, in ascending time order, with positions clamped to 0..1.

## Cursor::at

```rust
pub fn at(&mut self, t_ms: u32) -> FramePoint
```

Returns the smoothed cursor position in frame-local pixels at event-time `t_ms`.

### Inputs

- `t_ms: u32` - the frame's event time in milliseconds. *Why:* the exporter passes the frame timestamp so the tracker can advance its internal index forward and interpolate between surrounding samples. Must be non-decreasing across calls (the index only advances).

### Returns

`FramePoint { x, y }` in frame-local integer pixels after low-pass smoothing. Always within the frame because `raw_at` returns the frame centre before the first event and `to_frame` clamps to frame bounds.

### Implementation

1. Advance `self.idx` forward while `events[idx + 1].t <= t_ms`. *Why one event at a time:* frames are queried in ascending order; this is amortised O(1) over the full export rather than O(log n) per frame.
2. Call `raw_at(t_ms)` to get the interpolated, un-smoothed position.
3. If `!self.primed`, set `sx = raw.x`, `sy = raw.y`, `primed = true`. *Why seed on first call:* prevents a large artificial lerp from (0, 0) to the true starting position at the opening frame.
4. Otherwise apply exponential low-pass: `sx += (raw.x - sx) * 0.35` (same for `y`). The constant `A = 0.35` was chosen so small jitter (< a few pixels per sample) is largely suppressed while large deliberate movements converge within 4-5 frames (~66 ms at 60 fps). *Why low-pass over raw positions:* recorded mouse data is typically throttled to 60-100 Hz and exhibits sample-to-sample jitter that would produce a visibly shaking cursor in the rendered video.
5. Return `FramePoint { x: sx.round(), y: sy.round() }`.

### Behaviors

- `center_before_first_event` - querying at t=0 before the first event returns frame centre (960, 540 for a 1920x1080 screen).
- `smooths_toward_a_jump_target` - repeatedly querying against a held final position converges to within 2 pixels of the target.

## cursordraw

Submodule (`cursor/cursordraw.rs`). CPU rasterizer for the Enhanced synthetic cursor: sprite placement, click-bounce scale animation, and motion trail blending, clipped to the screen panel. Key items: `CursorSprite`, `decode_sprite`, `bounce_scale`, `draw_cursor`, `apply_enhanced` - full per-symbol docs in `cursor/cursordraw.md`.

## cursorset

Submodule (`cursor/cursorset.rs`). Manages the per-type cursor sprite set: decodes each shape once at prep time, inverts RGB for dark themes, and dispatches per-frame draw calls with panel-proportional sizing. Key items: `SPRITES`, `CursorPrep`, `prep`, `sprite_for`, `draw`, `invert_rgb` - full per-symbol docs in `cursor/cursorset.md`.

## cursorpreview

Submodule (`cursor/cursorpreview.rs`). Editor-preview cursor commands: expose the Capitaine sprite pack + cursor-type track to the frontend so the canvas preview draws the same cursor the export renders. Key items: `cursor_sprites`, `cursor_kinds` (Tauri commands) - full per-symbol docs in `cursor/cursorpreview.md`.
