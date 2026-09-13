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
    a: f32,
    idealize: f32,
    anchors: Vec<(u32, f32, f32)>,
    primed: bool,
}
```

Stateful cursor tracker that owns the mouse log and screen geometry.

- `events` - the full mouse log in ascending time order. *Why owned (not borrowed):* the tracker lives inside `FrameRenderer` for the whole render; a borrow would make the renderer self-referential. Owning the single copy also lets FX rendering read it via `events()` instead of a second copy of the log.
- `screen` - capture geometry (width, height, origin). *Why:* `raw_at` calls `to_frame`, which needs the screen rect to convert screen-space coordinates to frame-local pixels.
- `idx` - the index of the last event at or before the most recently queried time. *Why mutable:* enables a forward-only scan rather than a binary search on every frame, amortising cost to O(1) per call when frames are queried in order.
- `sx`, `sy` - smoothed position accumulators in frame-local float pixels. *Why floats:* the low-pass filter accumulates sub-pixel movement; rounding only happens on output.
- `a` - the low-pass alpha (follow smoothness). *Why a field, not a constant:* it is settings-driven (`CursorSettings::follow_alpha`, from the editor's Cursor Smoothness slider) - lower `a` = a smoother, more deliberate glide. `set_a` updates it live so a settings change reflects via `FrameRenderer::reload_edit` without rebuilding the cursor.
- `idealize` - path-idealization strength (0 = raw path, 1 = clean eased strokes between the anchors). Settings-driven (`CursorSettings::path_idealize`, the editor's Path Idealization slider); `set_idealize` updates it live.
- `anchors` - `(t, x, y)` frame-local anchors (the first sample, every click, and the last sample) that the idealized path eases between, so a meandering real route becomes deliberate strokes to the clicks. Precomputed once in `new` by `compute_anchors`.
- `primed` - whether `sx`/`sy` have been initialised. *Why:* the first call must seed the accumulator from the raw position rather than lerp toward it from (0, 0), which would produce an unwanted glide-in at the start of the recording.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer` owns one `Cursor`, calls `at` once per rendered frame to place the synthetic cursor / anchor effects, and exposes `events()` to the FX layer.

## Cursor::new

```rust
pub fn new(events: Vec<MouseEvent>, screen: ScreenInfo, a: f32) -> Self
```

Constructs a `Cursor` that takes ownership of the log, with all smoothing state at zero and the given follow alpha.

### Inputs

- `events: Vec<MouseEvent>` - the full mouse log, assumed sorted ascending by `t`. *Why ascending:* the forward-only `idx` scan skips unseen events; out-of-order events would cause missed samples. *Why by value:* the renderer moves its single copy of the log in here.
- `screen: ScreenInfo` - capture geometry, owned for the tracker's lifetime so `raw_at` can convert on every frame.
- `a: f32` - the follow low-pass alpha (`CursorSettings::follow_alpha`). *Why passed in:* the smoothness is a user setting, not a constant; the renderer derives it from `settings.cursor.smoothness`.

### Returns

`Cursor` with `idx = 0`, `sx = 0.0`, `sy = 0.0`, the given `a`, and `primed = false`.

## Cursor::set_a

```rust
pub fn set_a(&mut self, a: f32)
```

Updates the follow-smoothing alpha in place. *Why it exists:* `FrameRenderer::reload_edit` calls it so a Cursor Smoothness settings change takes effect on the cheap edit-reload path, without rebuilding the whole cursor (which would re-decode the log).

## Cursor::set_idealize

```rust
pub fn set_idealize(&mut self, s: f32)
```

Updates the path-idealization strength (clamped 0..1) in place. Like `set_a`, `FrameRenderer::reload_edit` calls it so a Path Idealization settings change takes effect on the cheap edit-reload path. At `at` time, when `idealize > 0` the smoothed position is blended toward `anchored_at` (the eased position along the click/endpoint anchor line).

## Cursor::events

```rust
pub fn events(&self) -> &[MouseEvent]
```

Borrows the owned mouse log. *Why it exists:* `FrameRenderer` keeps a single copy of the events inside its `Cursor`; FX rendering needs the same slice, so it reads it through this accessor rather than the renderer holding a second `Vec`.

### Returns

`&[MouseEvent]` - the events passed to `new`, in ascending time order.

## Cursor::screen

```rust
pub fn screen(&self) -> ScreenInfo
```

Returns the recording's `ScreenInfo` (`Copy`, so this is a cheap read, not a second owner). *Why it exists:* `Cursor` already applies `coordmap::to_frame(&self.screen, ...)` to convert its own raw mouse points to screen-local (`clicks`, `compute_anchors`); FX rendering (`fx_state::render`) needs the same origin to convert click-hit coordinates the same way, so `FrameRenderer::composite_at` reads it through this accessor rather than the renderer storing a second copy of `ScreenInfo`.

### Returns

`ScreenInfo` - the value passed to `new`.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::composite_at` passes `&self.cursor.screen()` to `fx_state::render`.

## Cursor::clicks

```rust
pub fn clicks(&self) -> Vec<(u32, f32, f32)>
```

Click (mouse-down) positions as `(event_time_ms, x, y)` where `x`/`y` are 0..1 fractions of the screen content - the same coordinate basis as the smoothed cursor (`to_frame`, then divide by the screen size). *Why it exists:* the editor preview draws click ripples; sharing the cursor's basis means a ripple lands exactly where the cursor clicked. `FrameRenderer::click_track` shifts these event times to output time.

### Returns

`Vec<(u32, f32, f32)>` - one tuple per `EventKind::Down` event, in ascending time order, with positions clamped to 0..1.

## Cursor::at

```rust
pub fn at(&mut self, t_ms: u32, dt_ms: f32) -> FramePoint
```

Returns the smoothed cursor position in frame-local pixels at event-time `t_ms`.

### Inputs

- `t_ms: u32` - the frame's event time in milliseconds. *Why:* the exporter passes the frame timestamp so the tracker can advance its internal index forward and interpolate between surrounding samples. Must be non-decreasing across calls (the index only advances).
- `dt_ms: f32` - the caller's **exact** frame period (`render::OUT_STEP_MS` = 16.666667 at 60fps; the exporter computes `1000 / out_fps` from its settings-resolved rate). *Why an argument and not `t_ms` minus the previous `t_ms`:* the frame loops build `t_ms` as `k * 1000 / out_fps` in integer math, so differencing it reads 16/17/17/16 at 60fps - the clock's rounding, not a real timing difference - and a first-order filter turns that straight into a per-frame ripple. Same contract, same reason, as `CameraSim::step`'s `dt_ms` (`export/render/mod.md`).

### Returns

`FramePoint { x, y }` in frame-local integer pixels after low-pass smoothing. Always within the frame because `raw_at` returns the frame centre before the first event and `to_frame` clamps to frame bounds.

### Implementation

1. Advance `self.idx` forward while `events[idx + 1].t <= t_ms`. *Why one event at a time:* frames are queried in ascending order; this is amortised O(1) over the full export rather than O(log n) per frame.
2. Call `raw_at(t_ms)` to get the interpolated, un-smoothed position.
3. If `!self.primed`, set `sx = raw.x`, `sy = raw.y`, `primed = true`. *Why seed on first call:* prevents a large artificial lerp from (0, 0) to the true starting position at the opening frame.
4. Otherwise apply exponential low-pass: `sx += (raw.x - sx) * follow::damping(self.a, dt_ms)` (same for `y`). `a` is the settings-driven follow alpha (`CursorSettings::follow_alpha`, default ~0.36) read as "fraction of the remaining error closed in one **60fps frame**": higher = snappier follow, lower = a smoother, more deliberate glide (the editor's Cursor Smoothness slider). *Why low-pass over raw positions:* recorded mouse data is typically throttled to 60-100 Hz and exhibits sample-to-sample jitter that would produce a visibly shaking cursor in the rendered video. *Why the `damping` conversion (H4 in the camera probe):* the raw alpha was applied once per STEP, so the same setting meant a different time constant at every output rate - ~4% off between a 16ms grid and the export's 16.667ms one, and ~100% off at a 30fps export. `export::camera::follow::damping` (`export/camera/follow.md`) turns the per-60fps-frame fraction into the equivalent fraction for this step, `1 - (1 - a)^(dt / 16.667)`, and is the **same** function the camera's own follow lerp uses - one definition, so the two filters cannot drift apart on what `smoothness` means. At exactly 60fps it returns `a` untouched, so the shipped trajectory is bit-identical (`jank_filter_tests::smoothing_off_is_bit_identical` did not move when this landed).
5. Return `FramePoint { x: sx.round(), y: sy.round() }`.

### Behaviors

- `center_before_first_event` - querying at t=0 before the first event returns frame centre (960, 540 for a 1920x1080 screen).
- `smooths_toward_a_jump_target` - repeatedly querying against a held final position converges to within 2 pixels of the target.

## cursordraw

Submodule (`cursor/cursordraw.rs`). CPU rasterizer for the Enhanced synthetic cursor: sprite placement, click-bounce scale animation, and motion trail blending, clipped to the screen panel. Key items: `CursorSprite`, `decode_sprite`, `bounce_scale`, `draw_cursor`, `draw_cursor_posed`, `apply_enhanced` - full per-symbol docs in `cursor/cursordraw.md`.

## cursorset

Submodule (`cursor/cursorset.rs`). Manages the per-type cursor sprite set: decodes each shape once at prep time, inverts RGB for dark themes, and dispatches per-frame draw calls with panel-proportional sizing. Key items: `SPRITES`, `CursorPrep`, `prep`, `sprite_for`, `posed` (the busy animation's per-frame sprite + transform), `draw`, `frame_placement` (the projection both cursor paths share), `invert_rgb` - full per-symbol docs in `cursor/cursorset.md`.

## busy

Submodule (`cursor/busy.rs`). Pack format v2's animated busy cursor: pure math turning an output-clock timestamp into "which frame, rotated how far, scaled how much". Key items: `BusyAnim` (spin/flip/pulse), `BusySpec` (a pack's declared animation plus its explicit frame count), `BusyPose`, `busy_pose` - full per-symbol docs in `cursor/busy.md`. Mirrored in TS by `src/editor/stage/cursorBusy.ts`.

## cursorxform

Submodule (`cursor/cursorxform.rs`). The rotated/scaled blit the animated busy cursor needs: destination-driven inverse mapping with premultiplied bilinear sampling, reached only when `BusyPose::is_identity` is false so every still cursor keeps `cursordraw`'s nearest-neighbour fast path. Key item: `blit_transformed` - full per-symbol docs in `cursor/cursorxform.md`.

## packdirs

Submodule (`cursor/packdirs.rs`). Where cursor packs live: the embedded set, the BUNDLED folders under the app's `assets/cursorpacks` resources, and the user's imports under `cursors_dir()`. Resolves a pack id to a folder (bundled wins) and explains why the exe-relative candidates make `tauri dev` work with no staging step. Key items: `cursors_dir`, `pack_dir`, `resource_candidates`, `bundled_root`, `default_pack_dir` (the embedded pack's own `assets/cursors` folder), `bundled_pack_dir`, `bundled_pack_dirs`, `imported_pack_dirs`, `resolve_pack_dir` - full per-symbol docs in `cursor/packdirs.md`.

## packlist

Submodule (`cursor/packlist.rs`). What packs exist and how the Cursor panel's grid shows them - names, folders, and a kind-to-filename map that already carries the embedded pack's `pointer.png` alias and the busy-is-arrow substitution, so the frontend needs neither rule. Key items: `CursorPackInfo`, `list_packs`, `imported_info`, `pack_files`, `default_filename` - full per-symbol docs in `cursor/packlist.md`.

## captured

Submodule (`cursor/captured.rs`). The REAL OS cursor, composited from the layer the recorder captured (`events::track::cursorlayer`) - what "System" means on any recording made since screen capture went cursor-free. Draws the actual recorded bitmap at the raw recorded point, with none of the Enhanced polish. Key items: `draws_captured` (the single gate that picks captured over synthetic), `CapturedCursors`, `CapturedCursors::load`, `CapturedCursors::sprite_at`, `CapturedCursors::draw` - full per-symbol docs in `cursor/captured.md`.

## cursorpreview

Submodule (`cursor/cursorpreview.rs`). Editor-preview cursor commands: expose the Capitaine sprite pack + cursor-type track to the frontend so the canvas preview draws the same cursor the export renders. Key items: `cursor_sprites` (now returning a `CursorPackDto` carrying the pack's busy animation), `cursor_kinds`, `cursor_layer` (Tauri commands) - full per-symbol docs in `cursor/cursorpreview.md`.
