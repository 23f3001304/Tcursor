# src-tauri/src/export/fx/click/hotkeycap.rs

Renders keyboard-shortcut chord captions (the HOTKEY overlay, not the spoken-caption track - see `export/fx/caption/captionlayout.rs`) onto the BGRA frame. Captions are centered in the bottom band of the frame with a 1px drop shadow for legibility on any background. Used by both the CPU and GPU FX paths; caption rendering is always CPU-only. The module also handles the logic for which action maps to which caption string and when the caption expires. Renamed from `caption.rs` (ADDED-3, M5 T1): keeping that name would make every later spoken-caption file in this milestone ambiguous.

**The glyph loop no longer lives here.** This module used to own the embedded face and the BGRA blend as `pub(crate)` items (`FONT` and `put`) because the spoken-caption blit borrowed them; they, and the `ab_glyph` outline loop that used them, now live in `export/fx/glyph.rs`, which a third overlay (the animated text items) also draws through. This file keeps only the part that is its own: which chord to show, for how long, and where in the frame it sits. It no longer imports `ab_glyph` at all, and all four of its tests pass unchanged across the move, which is what makes the move provably behaviour free.

## overlay

```rust
pub fn overlay(out: &mut [u8], ow: u32, oh: u32, actions: &[ActionEvent],
    keys: &HotkeySettings, et: u32, enabled: bool)
```

Top-level per-frame entry point: queries `caption_at` and, when a caption is active, delegates to `draw_caption`.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame to draw onto. *Why mutable:* the caption is blended in place on top of the full composite.
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* forwarded to `draw_caption` for centering and baseline positioning.
- `actions: &[ActionEvent]` - the action event log for this recording. *Why:* `caption_at` scans this to find the most recent triggering action within the display window.
- `keys: &HotkeySettings` - the user's configured key bindings. *Why:* the caption shows the actual bound chord (e.g. "Ctrl+1") not a generic label, so the display text is read from `keys` at render time.
- `et: u32` - current frame event-time in milliseconds. *Why:* passed to `caption_at` to determine which action (if any) falls within the 1300 ms display window.
- `enabled: bool` - whether caption overlay is on. *Why:* returns immediately when false, skipping both the action scan and the glyph render entirely.

### Returns

`()`. No-op when `enabled` is false or when `caption_at` returns `None`.

### Used by

- `src-tauri/src/export/fx/fx_state.rs` - `FxState::apply_post` calls `caption::overlay` as the final compositing step on both the CPU and GPU paths; it is the last thing written before the frame is sent to the encoder.

## caption_at

```rust
pub fn caption_at(actions: &[ActionEvent], keys: &HotkeySettings, et: u32, life_ms: u32) -> Option<(String, f32)>
```

Determines the caption string and its fade alpha for `et`, or `None` if no caption is active.

### Inputs

- `actions: &[ActionEvent]` - the action event log. *Why:* scanned for the most recent event within `life_ms` of `et`.
- `keys: &HotkeySettings` - the user's hotkey bindings. *Why:* the caption text is taken from the bound chord string rather than a hard-coded label, so the caption always matches what the user actually pressed.
- `et: u32` - query time in milliseconds. *Why:* defines the observation window `[et - life_ms, et]`.
- `life_ms: u32` - caption display lifetime in milliseconds. *Why caller-supplied:* `overlay` passes the module-level constant `CAP_MS = 1300`; the parameter allows `caption_at` to be called in tests with shorter windows.

### Returns

`Some((text, alpha))` where `text` is the chord string and `alpha` is `(1.0 - (et - a.t) / life_ms).clamp(0, 1)` - linear fade over the display window. `None` when: no action falls within the window; or the most-recent action is a `*HoldEnd` variant (hold-release explicitly clears the caption).

### Implementation

1. Filter `actions` to those where `a.t <= et && et - a.t < life_ms`, take the last (most recent). Return `None` if none.
2. Return `None` if the action kind is `ZoomHoldEnd`, `SpotlightHoldEnd`, or `VideoFxHoldEnd`. *Why:* releasing a hold should clear the caption immediately, not leave it fading for the full window.
3. Map the action kind to a display string from `keys`:
   - `ZoomHoldStart` - `keys.zoom_hold`
   - `SpotlightHoldStart` - `keys.spotlight_hold`
   - `VideoFxHoldStart` - `keys.video_fx_hold`
   - `SetLayout(id)` - `keys.layout_screen` / `layout_camera` / `layout_presenter` / `layout_screen_only` / `layout_camera_only`
4. Compute `p = (et - a.t) / life_ms`; return `Some((text, (1.0 - p).clamp(0, 1)))`.

### Behaviors

- `shows_bound_chord_within_window` - a `SetLayout(Screen)` at t=100 queried at t=200 with life=1300 returns `(keys.layout_screen, alpha > 0)`.
- `none_after_window_and_for_hold_end` - expired actions and `ZoomHoldEnd` both return `None`.
- `most_recent_action_wins` - when two actions are in the window, the text of the later one is returned.

## draw_caption

```rust
pub fn draw_caption(out: &mut [u8], ow: u32, oh: u32, text: &str, alpha: f32)
```

Rasterizes `text` into the bottom band of the frame, centered horizontally, through `glyph::draw_run` with a 1px drop shadow.

### Inputs

- `out: &mut [u8]` - the composited BGRA frame. *Why mutable:* glyph coverage samples are alpha-blended over the existing pixels.
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* used for horizontal centering (`x = (ow - width) / 2`) and baseline placement (`baseline = oh - oh * 0.06`).
- `text: &str` - the chord string to render (e.g. "Ctrl+Alt+1"). *Why:* the caller resolves the action-to-string mapping; this function is purely typographic.
- `alpha: f32` - overall caption opacity 0..1, typically the fade value from `caption_at`. *Why:* allows a smooth fade-out without computing a new font layout each frame.

### Returns

`()`. No-op when `text` is empty or `alpha <= 0.0`. Returns silently if the shared embedded font cannot be parsed.

### Implementation

1. Guard: return immediately if `text.is_empty() || alpha <= 0.0`.
2. `glyph::font()` for the shared Inter SemiBold face; return silently on `None`.
3. Compute `px = max(oh * 0.030, 8.0)` as the font size in pixels. *Why 3% of frame height:* at 1080p this is ~32 px, readable without dominating the frame; the minimum of 8 px prevents invisible text on tiny frames.
4. `x = (ow - glyph::run_width(&font, px, text)) / 2.0`, which centres the run horizontally on real Inter advances.
5. One `glyph::draw_run` at `(x, baseline)` where `baseline = oh - oh * 0.06`, in white, at `alpha`, with the shadow ON. That single call is what the seven steps above used to be; the left-edge and baseline convention and the one pixel dark shadow at 0.6 of the run's alpha are documented on `glyph::draw_run`.

### Behaviors

- `draw_caption_paints_pixels_and_noop_on_empty` - "Ctrl+Alt+1" at alpha 1.0 writes at least one non-zero byte; an empty string leaves the buffer all zeros.
