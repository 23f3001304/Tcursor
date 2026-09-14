# src-tauri/src/export/cursor/mod.rs

The cursor's per-frame position: the raw interpolated recording, or - when the Cursor panel asks for any polish - the route `path::PathModel` draws between the points the recording actually rested and clicked at. Owns its event log so it can live inside the `FrameRenderer` as the single owner of the events (also the source the FX layer reads). Since 2026-09-14 the polish is a MODEL of the whole recording rather than a filter on it: the old exponential low-pass lagged the real cursor (a click landed before the drawn cursor arrived), and the old idealization drew straight strokes between clicks only, ignoring every rest without one. Both now leave every rest and click exactly where it was (`path.md`).

## Cursor

```rust
pub struct Cursor {
    events: Vec<MouseEvent>,
    screen: ScreenInfo,
    idx: usize,
    path: path::PathModel,
    smooth: f32,
    ideal: f32,
    tilt: tilt::Tilt,
    tilt_max: f32,
}
```

- `events` - the full mouse log in ascending time order. *Why owned (not borrowed):* the tracker lives inside `FrameRenderer` for the whole render; a borrow would make the renderer self-referential. Owning the single copy also lets FX rendering read it via `events()` instead of a second copy of the log.
- `screen` - capture geometry (width, height, origin). *Why:* `raw_at` calls `to_frame`, which needs the screen rect to convert screen-space coordinates to frame-local pixels.
- `idx` - the index of the last event at or before the most recently queried time, for the raw path. *Why mutable:* a forward-only scan rather than a binary search on every frame, amortising to O(1) per call when frames are queried in order (`reset` rewinds it).
- `path` - the offline rest/move model (`path::PathModel`), built once from the log in `new`. Stateless between calls: a lookup is a binary search by time, so the preview's rewind-and-rescan needs nothing from it.
- `smooth` - `CursorSettings::smoothness`, 0..1: how glassy the glide between rests is (0 = the recording's own timing). `set_smoothness` updates it live.
- `ideal` - `CursorSettings::path_idealize`, 0..1: how straight the route between rests is (0 = the raw route). `set_idealize` updates it live.
- `tilt` - the motion-lean filter (`cursor/tilt.md`), fed the same position this struct hands back so the lean rides the path the cursor is actually drawn on. *Why it lives here and not in the draw:* it is stateful and must advance exactly once per frame, which is a property only the per-frame position lookup has.
- `tilt_max` - that filter's cap in degrees (`tilt::max_deg` of `CursorSettings::tilt`); `0.0` means the filter is off and `at` skips it entirely. *Why the derived cap rather than the raw setting:* `at` runs per frame and the clamp/scale belongs at the setter, not in the loop.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer` owns one `Cursor`, calls `at` once per rendered frame to place the synthetic cursor / anchor effects, and exposes `events()` to the FX layer.

## Cursor::new

```rust
pub fn new(events: Vec<MouseEvent>, screen: ScreenInfo, smoothness: f32) -> Self
```

Constructs a `Cursor` that takes ownership of the log, builds its path model, and starts with the given glide strength, no idealization and the tilt off.

### Inputs

- `events: Vec<MouseEvent>` - the full mouse log, assumed sorted ascending by `t`. *Why ascending:* the forward-only `idx` scan skips unseen events; out-of-order events would cause missed samples (the path model drops them). *Why by value:* the renderer moves its single copy of the log in here.
- `screen: ScreenInfo` - capture geometry, owned for the tracker's lifetime so `raw_at` can convert on every frame.
- `smoothness: f32` - `CursorSettings::smoothness_at` (0..1; 0 in plain-OS mode, which is the raw path). *Why passed in:* it is a user setting, not a constant.

### Returns

`Cursor` with `idx = 0`, the model built, `ideal = 0.0`, and the tilt filter off (`tilt_max = 0.0`) until `set_tilt` says otherwise.

## Cursor::reset

```rust
pub fn reset(&mut self)
```

Rewinds the forward-only sample index **and** the tilt (`Tilt::reset`). *Why the tilt goes with it:* `FrameRenderer::snap_cursor` calls this at a cut, where the viewer never saw the frames the filter would lean across - a cursor arriving on the far side of a splice still leaning from the gesture before it would read as a glitch. The path model needs no rewind: it is a function of time (`a_rewound_cursor_answers_like_a_fresh_one`).

## Cursor::set_smoothness

```rust
pub fn set_smoothness(&mut self, s: f32)
```

Updates the glide strength (clamped 0..1) in place. *Why it exists:* `FrameRenderer::reload_edit` calls it so a Cursor Smoothness settings change takes effect on the cheap edit-reload path, without rebuilding the whole cursor (which would re-decode the log). The path model rebuilds only its cached strokes, lazily, on the next lookup.

## Cursor::set_idealize

```rust
pub fn set_idealize(&mut self, s: f32)
```

Updates the path-idealization strength (clamped 0..1) in place. Like `set_smoothness`, `FrameRenderer::reload_edit` calls it so a Path Idealization settings change takes effect on the cheap edit-reload path. Above 0, every move's polished route is pulled toward the straight chord between the rests it joins (`path.md`).

## Cursor::set_tilt

```rust
pub fn set_tilt(&mut self, t: f32)
```

Updates the motion-tilt strength (`CursorSettings::tilt`, 0..1) in place, storing it as the derived cap `tilt::max_deg(t)`. Live-applied by `FrameRenderer::reload_edit` alongside `set_smoothness`/`set_idealize`, and set from `CursorSettings::tilt_at` in `FrameRenderer::new` so plain-OS mode gets 0.

Turning it off (`t <= 0`) also calls `Tilt::reset`. *Why:* without it, dragging the slider to 0 mid-lean would freeze the sprite at whatever angle it happened to hold - the setting must mean "upright", not "stop updating".

## Cursor::tilt_deg

```rust
pub fn tilt_deg(&self) -> f32
```

This frame's motion lean in degrees, clockwise-positive. Valid **after** `at`, which is what advances the filter - `FrameRenderer::composite_at` reads it on the same frame it just stepped and hands it to `cursorset::draw` and to `fx_lensbuild::LensFrame`, so the sprite, its glass lens and the frame that lens bends all tip by the same amount.

*Why an accessor rather than a field on `FramePose`:* the pose is built by `step_camera` and consumed by callers that never draw a cursor (`camera_track`, the jank probe); the lean is only ever wanted at the moment of drawing, by a caller that already holds the renderer.

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

Returns the recording's `ScreenInfo` (`Copy`, so this is a cheap read, not a second owner). *Why it exists:* `Cursor` already applies `coordmap::to_frame(&self.screen, ...)` to convert its own raw mouse points to screen-local (`clicks`, `path::PathModel::new`); FX rendering (`fx_state::render`) needs the same origin to convert click-hit coordinates the same way, so `FrameRenderer::composite_at` reads it through this accessor rather than the renderer storing a second copy of `ScreenInfo`.

### Returns

`ScreenInfo` - the value passed to `new`.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::composite_at` passes `&self.cursor.screen()` to `fx_state::render`.

## Cursor::clicks

```rust
pub fn clicks(&self) -> Vec<(u32, f32, f32)>
```

Click (mouse-down) positions as `(event_time_ms, x, y)` where `x`/`y` are 0..1 fractions of the screen content - the same coordinate basis as the drawn cursor (`to_frame`, then divide by the screen size). *Why it exists:* the editor preview draws click ripples; sharing the cursor's basis means a ripple lands exactly where the cursor clicked - and since the path model pins every click, exactly where the cursor IS at that instant. `FrameRenderer::click_track` shifts these event times to output time.

### Returns

`Vec<(u32, f32, f32)>` - one tuple per `EventKind::Down` event, in ascending time order, with positions clamped to 0..1.

## Cursor::at

```rust
pub fn at(&mut self, t_ms: u32, dt_ms: f32) -> FramePoint
```

Returns the drawn cursor position in frame-local pixels at event-time `t_ms`.

### Inputs

- `t_ms: u32` - the frame's event time in milliseconds. Must be non-decreasing across calls for the RAW path (the index only advances); the polished path does not care.
- `dt_ms: f32` - the caller's **exact** frame period (`render::OUT_STEP_MS` = 16.666667 at 60fps; the exporter computes `1000 / out_fps` from its settings-resolved rate). Only the motion lean consumes it now: the path is a function of time, so it never lags and never depends on the output rate. Same contract, same reason, as `CameraSim::step`'s `dt_ms` (`export/render/mod.md`).

### Returns

`FramePoint { x, y }` in frame-local integer pixels. Frame centre before the first event.

### Implementation

1. Advance `self.idx` forward while `events[idx + 1].t <= t_ms`, and take `raw_at(t_ms)` - the interpolated, un-smoothed position (frame centre before the first event).
2. Before the first event, or with `smooth <= 0 && ideal <= 0` (plain-OS mode, or both sliders at 0): the raw position, verbatim - bit-identical to the recording (`no_polish_is_the_raw_interpolation_bit_for_bit`).
3. Otherwise `path.at(t_ms, smooth, ideal)` - the polished route, which still passes through every rest and click exactly (`path.md`); the raw position is the fallback for an empty log.
4. Step the motion-tilt filter (`Tilt::step`) with that FINAL position converted to reference px by `tilt::ref_scale`, unless `tilt_max` is 0, in which case the filter is skipped outright and costs nothing. *Why the final position:* an idealized stroke should lean into its own clean route, not the wander the viewer never sees.
5. Return `FramePoint { x, y }` rounded from that same final position.

### Behaviors (`mod_tests.rs`)

- `center_before_first_event` - querying at t=0 before the first event returns frame centre (960, 540 for a 1920x1080 screen).
- `arrives_with_the_recording_not_after_it` - a jump to a target held from `t=100` is ON the target at `t=100` at smoothness 0, 0.6 and 1 (the old low-pass converged frames later).
- `idealize_pulls_a_detour_toward_the_click_anchor_line` - a continuous throw arcing 800 px off the line between two clicks hugs the line at `ideal 1`, and the second click itself is exact.
- `no_polish_is_the_raw_interpolation_bit_for_bit` - smoothness 0 + idealize 0 equals `raw_at` at every frame.
- `a_thrown_cursor_leans_and_a_resting_one_does_not` - a 3 px/ms sweep leans past a degree, settles back upright once the cursor is held, and never leaves 0 while `set_tilt` has not been called.
- `a_cut_snaps_the_lean_away_with_the_position` - `reset` drops the lean.
- `a_rewound_cursor_answers_like_a_fresh_one` - a full sweep, `reset`, and the same sweep again are identical.

## path

Submodule (`cursor/path.rs`). The offline rest/move path model the polished cursor is drawn from: rests and clicks are the recording verbatim, moves between them are re-timed (Smoothness) and straightened (Path Idealization) with both ends pinned. Key items: `REST_MS`, `rest_px`, `PathModel`, `PathModel::new`, `PathModel::at` - full per-symbol docs in `cursor/path.md`.

## cursordraw

Submodule (`cursor/cursordraw.rs`). CPU rasterizer for the Enhanced synthetic cursor: sprite placement, click-bounce scale animation, and motion trail blending, clipped to the screen panel. Key items: `CursorSprite`, `decode_sprite`, `bounce_scale`, `draw_cursor`, `draw_cursor_posed`, `apply_enhanced` - full per-symbol docs in `cursor/cursordraw.md`.

## cursorset

Submodule (`cursor/cursorset.rs`). Manages the per-type cursor sprite set: decodes each shape once at prep time, inverts RGB for dark themes, and dispatches per-frame draw calls with panel-proportional sizing. Key items: `SPRITES`, `CursorPrep`, `prep`, `sprite_for`, `posed` (the busy animation's per-frame sprite + transform), `draw`, `frame_placement` (the projection both cursor paths share), `invert_rgb` - full per-symbol docs in `cursor/cursorset.md`.

## busy

Submodule (`cursor/busy.rs`). Pack format v2's animated busy cursor: pure math turning an output-clock timestamp into "which frame, rotated how far, scaled how much". Key items: `BusyAnim` (spin/flip/pulse), `BusySpec` (a pack's declared animation plus its explicit frame count), `BusyPose`, `busy_pose` - full per-symbol docs in `cursor/busy.md`. Mirrored in TS by `src/editor/stage/cursorBusy.ts`.

## tilt

Submodule (`cursor/tilt.rs`). The motion lean: a low-pass on the drawn cursor's velocity feeding a lightly under-damped spring, on a fixed 1 ms substep grid so a 30 fps export matches a 60 fps one. Key items: `REF_W`, `MAX_DEG`, `Tilt`, `Tilt::step`, `target_deg`, `max_deg`, `ref_scale` - full per-symbol docs in `cursor/tilt.md`. Mirrored in TS by `src/editor/stage/cursorTilt.ts`, pinned against the same five instants.

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
